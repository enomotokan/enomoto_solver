//! 列シングルトン消去 (Achterberg et al. 2019 / PaPILO の `ColSingleton`)。
//!
//! 実制約 (A の行と G の多変数行) にちょうど 1 回しか現れない変数 `x_j` が
//! 等式行に現れる場合、その行を `x_j` について解いて代入消去する:
//! 目的係数を行内の他変数へ振り替え、変数と行の両方を問題から落とす。
//! `x_j` の値は後処理で [`Substitution::value`] により復元する。
//!
//! G の単一変数行 (箱制約) は出現回数に数えない。候補は A の等式行のみ
//! (不等式行の場合は符号による場合分けが必要なので扱わない)。
//!
//! 行を落とすと `lb_j <= x_j <= ub_j` を強制するものが無くなるため、
//! `lb_j <= (rhs - r)/coeff <= ub_j` (`r` = 行の他項の和) を `r` についての
//! 最大 2 本の `<=` 行に変換し、呼び出し側が G/h に追加する。無限の側は
//! 空の制約になるので出力しない (両側無限の「自由列シングルトン」は置換行 0 本)。

use crate::presolve::propagate::GView;
use crate::sparse::{FaerCsr, csr_from_rows, csr_is_canonical, csr_rows};
use crate::params::presolve::{IMPLIED_TOL, SUBSTITUTION_PIVOT_RATIO, TOL};

/// 代入式 `x[var] = (rhs - Σ terms[k].1 * x[terms[k].0]) / coeff`。
/// 後処理で他変数の解から消去変数の値を復元するのに使う。
pub struct Substitution {
    /// 消去された変数の列番号。
    pub var: usize,
    /// 行内の他変数 `(列番号, 係数)`。
    pub terms: Vec<(usize, f64)>,
    /// 行の右辺。
    pub rhs: f64,
    /// 消去変数の係数 (ピボット)。
    pub coeff: f64,
}

impl Substitution {
    /// `x` 中の他変数の値から `x[self.var]` を計算して返す。
    /// `terms` が参照する全添字の値が既に確定していることが前提。
    pub fn value(&self, x: &[f64]) -> f64 {
        let mut rhs = self.rhs;
        for &(k, v) in &self.terms {
            rhs -= v * x[k];
        }
        rhs / self.coeff
    }
}

/// `colsingleton`/`doubleton` が、残りの項の箱制約から既に含意される
/// 境界保存行を省略するかどうか (既定で省略。`ENOMOTO_KEEP_IMPLIED_BOUND_ROWS`
/// を設定すると省略しない)。
pub(crate) fn skip_implied_bound_rows() -> bool {
    env_str!("ENOMOTO_KEEP_IMPLIED_BOUND_ROWS").is_none()
}

/// 各項の箱 `[lb_k, ub_k]` 上での `Σ v * x_k` の値域 `(最小, 最大)`。
/// NaN (∞-∞) は該当側の無限大として扱う。
pub(crate) fn terms_range(terms: &[(usize, f64)], lb: &[f64], ub: &[f64]) -> (f64, f64) {
    let (mut lo, mut hi) = (0.0f64, 0.0f64);
    for &(k, v) in terms {
        let (a, b) = if v > 0.0 { (v * lb[k], v * ub[k]) } else { (v * ub[k], v * lb[k]) };
        lo += a;
        hi += b;
    }
    (if lo.is_nan() { f64::NEG_INFINITY } else { lo }, if hi.is_nan() { f64::INFINITY } else { hi })
}

/// 列シングルトン消去 1 パスの結果。
pub struct EliminationResult {
    /// 消去した行を除いた新しい等式行列 A。
    pub a: FaerCsr,
    /// 新しい等式右辺 b。
    pub b: Vec<f64>,
    /// 目的係数を振り替えた後の c (消去変数の係数は 0)。
    pub c: Vec<f64>,
    /// 呼び出し側が G に追加すべき `<=` 行: 消去変数の箱制約を
    /// 残りの変数に写したもの (正しさのため必須)。
    pub extra_g_rows: Vec<Vec<(usize, f64)>>,
    /// `extra_g_rows` に対応する右辺 h。
    pub extra_h: Vec<f64>,
    /// 行った代入 (後処理で逆順に適用する)。
    pub substitutions: Vec<Substitution>,
}

// 試作 `ENOMOTO_MIP_INTSING` の `h` (保留、analysis/slowm_20261119 §13。はじめ `o` と呼んだ形): 整数最適化の外側の前処理 (`presolve_mip` の最上段) の間は、
// 費用を移すと費用 ≥ 0 の列の費用が負になる整数列だけ INTSING で消さずに数え (状態 1)、支配される 0-1 列の固定 (`ENOMOTO_MIP_DOMCOL`) を
// 済ませてから、かけ直しの前処理で消す (状態 2)。消す列の費用を行の他の列へ移すと費用が負の列ができ、支配の逆向きの経路 (費用 ≥ 0 が要る) が使えなくなる (air03: 固定 2297 → 160 本)
thread_local! {
    pub(crate) static INTSING_HOLD: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
    pub(crate) static INTSING_HELD: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// `ENOMOTO_MIP_INTSING` に `h` が入っているか (既定はオフ)
pub(crate) fn intsing_order() -> bool {
    env_def!("ENOMOTO_MIP_INTSING", "1go").is_some_and(|v| v != "0" && v.contains('h'))
}

/// `ENOMOTO_MIP_INTSING` に `o` が入っているか (既定 `1go` に含む): 支配される 0-1 列の固定を、前処理の前の元の問題に先にかける (§13.4)
pub(crate) fn intsing_domfirst() -> bool {
    env_def!("ENOMOTO_MIP_INTSING", "1go").is_some_and(|v| v != "0" && v.contains('o'))
}

/// G を行列形式で受け取る版の列シングルトン消去 (1 パス、連鎖しない)。
/// 出現回数は入力から一度だけ数えるので、消去によって新たに生じた
/// シングルトンは次回の呼び出しで拾われる。
#[allow(dead_code)] // `run_extended` は `GView` 版を直接呼ぶ
pub fn eliminate_singleton_equalities(n: usize, a: &FaerCsr, b: &[f64], g: &FaerCsr, h: &[f64], c: &[f64]) -> EliminationResult {
    eliminate_singleton_equalities_view(n, a, b, GView::Mat { g, h }, c)
}

/// 列シングルトン消去の本体。G は [`GView`] のどちらの形式でもよい
/// (分割形式なら `lb`/`ub` と多変数行をそのまま使う)。
///
/// `n`: 変数数、`a`/`b`: 等式制約、`gv`: 不等式制約 (箱制約込み)、`c`: 目的係数。
pub fn eliminate_singleton_equalities_view(n: usize, a: &FaerCsr, b: &[f64], gv: GView<'_>, c: &[f64]) -> EliminationResult {
    let a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a);

    // 各変数の箱制約。G の多変数行はコピーせずその場で出現回数だけ数える。
    let (lb, ub) = gv.bounds(n);

    // 実制約 (A の行 + G の多変数行) での各変数の出現回数。G の単一変数行は箱制約なので数えない。
    let mut appearances = vec![0usize; n];
    // 各変数が最後に現れた A の行 (出現 1 回の変数ではその唯一の行)。
    let mut owning_a_row = vec![None; n];
    for (i, row) in a_rows.iter().enumerate() {
        for &(j, v) in row {
            if v != 0.0 {
                appearances[j] += 1;
                owning_a_row[j] = Some(i);
            }
        }
    }
    match gv {
        GView::Mat { g, .. } => {
            let gr = g.as_ref();
            for i in 0..gr.nrows() {
                let cols = gr.col_indices_of_row_raw(i);
                if cols.len() == 1 {
                    continue; // 箱制約行は数えない
                }
                for (&j, &v) in cols.iter().zip(gr.values_of_row(i)) {
                    if v != 0.0 {
                        appearances[j] += 1;
                    }
                }
            }
        }
        // 分割形式: rows は多変数行のみ。
        GView::Split { rows, .. } => {
            for row in rows {
                for &(j, _) in row {
                    appearances[j] += 1;
                }
            }
        }
    }

    // 代入に使って削除する A の行。
    let mut eliminated_rows = vec![false; a_rows.len()];
    let mut substitutions = Vec::new();
    let mut extra_g_rows = Vec::new();
    let mut extra_h = Vec::new();
    let mut new_c = c.to_vec();

    // 整数最適化の前処理中は整数列を消さない (復元値が整数になる保証がないため)。
    let int_mask = crate::presolve::int_mask();
    // `ENOMOTO_MIP_KEEP_BOUNDED_SLACK` (試作): 整数最適化の前処理で、上下限が両方とも有限の連続列は消さない (等式を範囲制約に
    // 変えない)。timtab1 は 171 本の等式すべてに有界の連続のスラック (目的の費用つき) があり、消すと等式が範囲制約になる
    // (HiGHS は残す: 161 行 360 列)
    let keep_bounded = int_mask.is_some() && env_str!("ENOMOTO_MIP_KEEP_BOUNDED_SLACK").is_some();
    // 試作 `ENOMOTO_MIP_ORDINV` の `d` ([`super::ordkey`]): 1 行に列シングルトンが複数あるとき、どれで行を使うかを列番号でなく列の鍵の順で決める
    let okeys = if super::ordkey::ordinv('d') { super::ordkey::keys() } else { None };
    let mut col_order: Vec<usize> = (0..n).collect();
    if okeys.is_some() {
        col_order.sort_by_key(|&j| super::ordkey::sort_key(&okeys, j));
    }
    // `ENOMOTO_MIP_INTSING` (2026-11-19 から既定 `1go`、`=0` で以前の動き。analysis/slowm_20261119 §13.5、docs/improvement_history.md): 整数列でも、等式 1 本にだけ現れ、その係数が ±1 で、
    // 行の他の列がすべて整数列・係数が整数・右辺が整数なら消す (復元値 `(rhs − Σ) / ±1` は整数)。SCIP の setppc・linear の
    // 多重集約に当たる。neos-1445765 は費用つきの 0-1 列 y_k が `Σ_{S_k} x_j − y_k = 0` の 1 本にだけ現れ (1000 本)、
    // SCIP は y_k を消して費用を x_j に移す (2146 → 1134 列)。当方でこの形を解くと 183.8 → 47.8 秒 (1 スレッド)
    let int_sing = int_mask.is_some() && env_def!("ENOMOTO_MIP_INTSING", "1go").is_some_and(|v| v != "0");
    // `d` (定義の形だけ): 行の他の列の係数の符号がすべて消す列の係数と逆 (`y = Σ a x`、a > 0 の形) のときだけ消す。air03 の
    // set partitioning 行 (`x_j + Σ = 1`、符号がそろう) の列シングルトン 118 本を消すと、費用が行の他の列へ配られて後の前処理
    // (支配列) が働かなくなり、0 ノードのまま 3.2〜5.1 → 4.7〜6.6 秒に遅くなった
    let int_sing_def = env_def!("ENOMOTO_MIP_INTSING", "1go").is_some_and(|v| v.contains('d'));
    // `g` (数え上げの一般整数列を残す): 消す列が 0-1 でなく (境界の幅 > 1)、費用があり、行の他の列の係数の符号がすべて消す列と逆
    // (`y = Σ a x`、a > 0 の形で y が「個数」を数える費用つきの一般整数列) なら消さない。この列で分枝できなくなるのが損の元と見る
    // (rocI-4-11・rococoC10-001000・rococoB10-011000 はこの形の列だけを 1〜4 本消し、M の 3〜8 種で 1.08・1.47 倍。0-1 列の同じ形
    // (neos-1445765、0.20 倍) と、符号の混ざる行 (hypothyroid-k1、0.60 倍) は消す)
    let int_sing_cnt = env_def!("ENOMOTO_MIP_INTSING", "1go").is_some_and(|v| v.contains('g'));
    // `s` (同符号の 0-1 列を残す): 消す列が 0-1 で、行の他の係数がすべて消す列と同符号 (`y + Σ a x = b`、a > 0 の形) なら消さない
    // (supportcase33・air03・traininstance6 の損はこの形だけ。M・P の 1g で解けた数 −3・air03 の素の時間 1.3〜1.5 倍。README §11.6〜§12)
    let int_sing_same = env_def!("ENOMOTO_MIP_INTSING", "1go").is_some_and(|v| v.contains('s'));
    let int_sing_hold = int_sing && INTSING_HOLD.with(|h| h.get()) == 1;
    let is_integral = |v: f64| (v - v.round()).abs() <= 1e-12 * (1.0 + v.abs());
    let intsing_log = int_sing && env_str!("ENOMOTO_MIP_INTSING_LOG").is_some();
    let mut intsing_stats: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut intsing_len = 0usize;
    for j in col_order {
        if appearances[j] != 1 {
            continue;
        }
        let is_int_col = int_mask.as_ref().is_some_and(|m| m[j]);
        if is_int_col {
            let ok = int_sing
                && owning_a_row[j].is_some_and(|i| {
                    let m = int_mask.as_ref().unwrap();
                    let row = &a_rows[i];
                    let aj = row.iter().find(|&&(k, _)| k == j).map_or(0.0, |&(_, v)| v);
                    let counting = int_sing_cnt && ub[j] - lb[j] > 1.5 && c[j] != 0.0 && row.iter().all(|&(k, v)| k == j || v * aj < 0.0);
                    let same_bin = int_sing_same && lb[j] == 0.0 && ub[j] == 1.0 && row.iter().all(|&(k, v)| k == j || v * aj > 0.0);
                    is_integral(b[i])
                        && !counting
                        && !same_bin
                        && row.iter().all(|&(k, v)| if k == j { (v.abs() - 1.0).abs() <= 1e-12 } else { m[k] && is_integral(v) && (!int_sing_def || v * aj < 0.0) })
                });
            if !ok {
                continue;
            }
            // 保留するのは、費用を移すと費用 ≥ 0 の列の費用が負になる列だけ (支配の逆向きの経路を壊す形)。ほかは今まで通りここで消す
            if int_sing_hold && new_c[j] != 0.0 && {
                let i = owning_a_row[j].unwrap();
                let aj = a_rows[i].iter().find(|&&(k, _)| k == j).map_or(0.0, |&(_, v)| v);
                a_rows[i].iter().any(|&(k, v)| k != j && new_c[k] >= 0.0 && new_c[k] - new_c[j] * v / aj < -1e-9 * (1.0 + new_c[k].abs()))
            } {
                INTSING_HELD.with(|h| h.set(h.get() + 1));
                continue;
            }
        }
        if keep_bounded && lb[j].is_finite() && ub[j].is_finite() {
            continue;
        }
        // 試作 `ENOMOTO_MIP_KEEP_COSTED_SINGLETON` (群 2、analysis/vsscip_slow_20261118/tree2/README.md 10 節): 整数最適化の前処理で、
        // 費用のついた連続の列シングルトンは消さない (費用を等式の他の列へ移さない)。neos-5107597-kakapo の完了時刻の列
        // 値 `c` なら、等式のほかの列がすべて連続のときだけ残す (費用が連続列に移る形。markshare_4_0 は費用が 0-1 列に移る形で、
        // 残すとノードが 26.5 万 → 43.2 万に増え 2〜3 倍遅い)
        let keep_costed = int_mask.is_some() && env_str!("ENOMOTO_MIP_KEEP_COSTED_SINGLETON").is_some();
        let keep_costed_cont = env_str!("ENOMOTO_MIP_KEEP_COSTED_SINGLETON") == Some("c");
        if keep_costed && c[j] != 0.0 && !keep_costed_cont {
            continue;
        }
        let Some(i) = owning_a_row[j] else { continue }; // 唯一の出現が G 側
        // (`c` はさらに上下限の片方が無限の列だけ。timtab1 の有界のスラック 6 本を残すと 1 スレッドで 44.6 → 64.2 秒)
        if keep_costed && keep_costed_cont && c[j] != 0.0 && !(lb[j].is_finite() && ub[j].is_finite()) && int_mask.as_ref().is_some_and(|m| a_rows[i].iter().all(|&(k, _)| k == j || !m[k])) {
            continue;
        }
        if eliminated_rows[i] {
            // この行は既に別のシングルトン列の消去に使われた (1 行で解けるのは 1 変数のみ)。
            // `j` は変数として残り、その代入式の項になる。
            continue;
        }
        let row = &a_rows[i];
        let coeff = row.iter().find(|&&(k, _)| k == j).unwrap().1;
        if coeff.abs() < TOL {
            continue;
        }
        // Markowitz 型のピボット判定: 行の最大絶対値に比べて小さすぎる係数で割ると、
        // 復元時に誤差が `max|row|/|coeff|` 倍に増幅されるので消去しない。
        let row_max = row.iter().map(|&(_, v)| v.abs()).fold(0.0f64, f64::max);
        // ただし整数最適化の前処理で、行の他の列がすべて整数列なら、復元に使う値は整数 (丸め済み) なので誤差は
        // 増幅されない。この場合は判定しない (qnet1: 目的値を定義する連続列 `s = 132.55 y1 + 934.75 y2` を消せる)
        let others_int = int_mask.as_ref().is_some_and(|m| row.iter().all(|&(k, _)| k == j || m[k])) && env_str!("ENOMOTO_MIP_CS_PIVOT_ALWAYS").is_none();
        // FJ の問題の前処理 (`ENOMOTO_MIP_FJ_P3`) では、費用のある連続列 (目的を定める列) は比の下限を
        // `ENOMOTO_T_MIP_CS_OBJ_PIVOT_RATIO` (既定 1e-6) まで下げる。physiciansched6-2 は目的の費用を 1 本の連続列
        // (`0.000999 y = Σ (罰則の列) + …` の 37 列の等式、行の最大係数 1) が持ち、この列が残ると Feasibility Jump が解を
        // 見つけられない (HiGHS は Free col substitution で消し、費用を行の列に配る)。復元の誤差は 1/0.000999 倍程度
        let ratio = if int_mask.is_some() && new_c[j] != 0.0 && crate::fj_p3_force() {
            tunable!("ENOMOTO_T_MIP_CS_OBJ_PIVOT_RATIO", 1e-6, f64)
        } else {
            tunable!("ENOMOTO_T_CS_SUBSTITUTION_PIVOT_RATIO", SUBSTITUTION_PIVOT_RATIO, f64)
        };
        if !others_int && coeff.abs() < ratio * row_max {
            continue;
        }
        // x_j 以外の項。
        let terms: Vec<(usize, f64)> = crate::sparse::collect_with_capacity(row.len(), row.iter().filter(|&&(k, _)| k != j).cloned());
        let rhs = b[i];

        // 目的係数 c_j を他変数へ振り替える: c_k -= c_j * a_ik / coeff。
        let cj = new_c[j];
        if cj != 0.0 {
            for &(k, a_ik) in &terms {
                new_c[k] -= cj * a_ik / coeff;
            }
            new_c[j] = 0.0;
        }

        // x_j の箱制約を r = Σterms に写す:
        // lb_j <= (rhs - r)/coeff <= ub_j  <=>  rhs-hi <= r <= rhs-lo
        // (lo/hi = min/max(coeff*lb_j, coeff*ub_j) で coeff の符号を吸収)。
        // 無限の側は自明な制約になるので出力しない。
        // 符号で向きを決める (min/max で並べ替えると、矛盾した境界 lb_j > ub_j が
        // 正常な区間に化けて実行不能性が失われる。矛盾したまま lo > hi で出せば、
        // 出力される 2 本の行が互いに矛盾し、後段が実行不能を検出する)。
        let extra_before = extra_g_rows.len();
        let (lo, hi) = if coeff >= 0.0 { (coeff * lb[j], coeff * ub[j]) } else { (coeff * ub[j], coeff * lb[j]) };
        // 残りの項の箱から既に含意される側は冗長なので省略する。
        // r_lo/r_hi: 項の箱から得られる r の値域。
        let (r_lo, r_hi) = if skip_implied_bound_rows() { terms_range(&terms, &lb, &ub) } else { (f64::NEG_INFINITY, f64::INFINITY) };
        if lo.is_finite() && !(r_hi <= rhs - lo + IMPLIED_TOL * (1.0 + (rhs - lo).abs())) {
            extra_g_rows.push(terms.clone());
            extra_h.push(rhs - lo);
        }
        if hi.is_finite() && !(r_lo >= rhs - hi - IMPLIED_TOL * (1.0 + (rhs - hi).abs())) {
            extra_g_rows.push(terms.iter().map(|&(k, v)| (k, -v)).collect());
            extra_h.push(hi - rhs);
        }

        // 診断 `ENOMOTO_MIP_INTSING_LOG` (analysis/slowm_20261119、振る舞いは変えない): INTSING で消した整数列の形を数える
        if is_int_col && intsing_log {
            let same = terms.iter().filter(|&&(_, v)| v * coeff > 0.0).count();
            let opp = terms.len() - same;
            let kind = if same == 0 { "opp" } else if opp == 0 { "same" } else { "mixed" };
            let bnd = if lb[j] == 0.0 && ub[j] == 1.0 { "bin" } else if lb[j].is_finite() && ub[j].is_finite() { "box" } else if lb[j].is_finite() || ub[j].is_finite() { "half" } else { "free" };
            let cst = if cj == 0.0 { "c0" } else { "cost" };
            let rows_added = extra_g_rows.len() - extra_before;
            *intsing_stats.entry(format!("{kind}/{bnd}/{cst}/rhs{}/add{rows_added}", if rhs == 0.0 { "0" } else { "≠0" })).or_insert(0usize) += 1;
            intsing_len += terms.len();
        }
        substitutions.push(Substitution { var: j, terms, rhs, coeff });
        eliminated_rows[i] = true;
    }
    if intsing_log && !intsing_stats.is_empty() {
        let n_int = int_mask.as_ref().map_or(0, |m| m.iter().filter(|&&b| b).count());
        let tot: usize = intsing_stats.values().sum();
        eprintln!("INTSING_LOG: eliminated {tot} integer singletons of {n_int} integer columns ({:.2}%), mean row len {:.1}: {:?}", 100.0 * tot as f64 / n_int.max(1) as f64, intsing_len as f64 / tot as f64, intsing_stats);
    }

    let mut new_a_rows = Vec::with_capacity(a_rows.len());
    let mut new_b = Vec::with_capacity(b.len());
    for (i, row) in a_rows.into_iter().enumerate() {
        if !eliminated_rows[i] {
            new_a_rows.push(row);
            new_b.push(b[i]);
        }
    }

    // 何も消去せず `a` が既に正準形なら、再構築せずそのまま複製する。
    let new_a = if substitutions.is_empty() && csr_is_canonical(a) { a.clone() } else { csr_from_rows(&new_a_rows, n) };
    EliminationResult {
        a: new_a,
        b: new_b,
        c: new_c,
        extra_g_rows,
        extra_h,
        substitutions,
    }
}
