//! 前処理 (presolve) パイプライン。単体法 (`simplex.rs`) と内点法
//! (`interior_point.rs`) が共通の入口 [`run_extended`] から同じ処理を使う。
//!
//! 問題は `A x = b`, `G x <= h` の形で扱う (変数の有限な上下限は G の
//! 単一変数行として含める。[`build_a_g`] 参照)。処理の流れ:
//!
//!   1. [`scaling::compute`] + [`scaling::apply`]: 修正 Ruiz 均衡化 (元データで 1 回)。
//!   2. [`redundancy`]: 重複・一次従属な等式行、重複・支配される不等式行の削除。
//!      不等式側 ([`redundancy::reduce_inequalities`]) は安価なので各外側ラウンドの
//!      末尾でも再実行する (代入により新たな重複行が生じうるため)。
//!   3. 外側ラウンドループ (最大 `rounds` 回): [`propagate::propagate`] (活動度に基づく
//!      上下限の強化) → 等式行の伝播 → [`dualfix`] → [`dualpropagate`] →
//!      [`foldfixed`] → 内側ループ ([`rowsingleton`] ↔ [`colsingleton`]、最初の内側パス
//!      のみ [`doubleton`]) → [`aggregator`] → [`parallelcols`] → 不等式行の重複削除。
//!      行数・上下限が変化しなくなった (不動点) 時点で打ち切る。一部の高コストな手法
//!      (`doubleton`/`dualpropagate`/`parallelcols`) は空振りが続くとそのラウンド以降停止する。
//!   4. 最終の [`propagate::propagate`] と自由変数消去 ([`freevar`])。
//!
//! 消去した変数は [`ExtendedPresolveResult::postsolve_log`] に時系列で記録され、
//! 呼び出し側が逆順に適用して元の変数値を復元する。
//!
//! 結果は上下限を分離した形 (`lb`/`ub`/`real_rows`/`real_rhs`、単体法用) と、
//! 上下限を G に戻した形 ([`ExtendedPresolveResult::g_h`]、内点法用) の両方で取り出せる。

pub mod aggregator;
pub mod colsingleton;
pub mod doubleton;
pub mod dominatedcol;
pub mod dualfix;
pub mod dualpropagate;
pub mod foldfixed;
pub mod freevar;
pub mod ineqsingleton;
pub mod parallelcols;
pub mod parallelrows;
pub mod propagate;
pub mod redundancy;
pub mod rowdominance;
pub mod rowsingleton;
pub mod scaling;
pub mod smallcoeff;
pub mod sparsify;
pub mod stuffing;

use crate::sparse::{FaerCsr, CsrRowBuilder, csr_from_rows, csr_row_vec, csr_rows};
use crate::types::{ConstraintRow, RowSense, VariableData};
use scaling::Scaling;
use crate::params::presolve::{
    DOUBLETON_STRIKES, DUALPROPAGATE_STRIKES, EQPROP_FIXPOINT, EQPROP_ROUNDS, OPPOSITE_PAIR_EQUALITY, PRESOLVE_FIXPOINT, PROPAGATION_FIXPOINT, PRESOLVE_WORK_BUDGET, EQPROP_SKIP_IDLE, FIXPOINT_RELTOL, INEQ_SINGLETON_LARGE, LARGE_PRESOLVE_MIN_ROWS, PARALLELCOLS_STRIKES, PRESOLVE_EXTRA_ROUNDS_LARGE, PROPAGATION_PASSES_LARGE, PRESOLVE_SPLIT_G, REDEQ_MODE,
    ROUND_STRUCT_STOP, ROUND_STRUCT_STOP_FIXPOINT,
};

/// モデルの変数・制約から `(A, b, G, h)` を組み立てる。
///
/// 等式行は A/b へ、`<=` 行はそのまま、`>=` 行は符号反転して G/h へ入れる。
/// 変数の有限な上下限は G の単一変数行 (`ub`: `(j, 1.0)`, `h=ub` /
/// `lb`: `(j, -1.0)`, `h=-lb`) として追加し、無限の上下限は行を作らない。
/// `interior_point::qp::build` と `simplex.rs` の前処理入口が共用する。
pub fn build_a_g(variables: &[VariableData], constraints: &[ConstraintRow]) -> (FaerCsr, Vec<f64>, FaerCsr, Vec<f64>) {
    // 高速版 (CsrRowBuilder へ直接書き込む) を試し、行が拒否されたら行リスト版で作り直す。
    // どちらも同じ行列をビット単位で生成する。
    if let Some(r) = build_a_g_direct(variables, constraints) {
        return r;
    }
    build_a_g_rows(variables, constraints)
}

/// [`build_a_g`] の高速版: 2 つの [`CsrRowBuilder`] に使い回しの行バッファで
/// 直接書き込む。範囲外の列などで行が拒否されたら `None`。
fn build_a_g_direct(variables: &[VariableData], constraints: &[ConstraintRow]) -> Option<(FaerCsr, Vec<f64>, FaerCsr, Vec<f64>)> {
    let n = variables.len();
    let (mut nnz_a, mut nnz_g, mut rows_a) = (0usize, 0usize, 0usize);
    for row in constraints {
        if matches!(row.sense, RowSense::Eq) {
            nnz_a += row.expr.coeffs.len();
            rows_a += 1;
        } else {
            nnz_g += row.expr.coeffs.len();
        }
    }
    // 上下限行の総数 (有限な ub と lb の個数)。
    let n_bounds = variables.iter().map(|v| v.ub.is_finite() as usize + v.lb.is_finite() as usize).sum::<usize>();
    let rows_g = constraints.len() - rows_a + n_bounds;
    let mut a = CsrRowBuilder::with_capacity(n, rows_a, nnz_a);
    let mut g = CsrRowBuilder::with_capacity(n, rows_g, nnz_g + n_bounds);
    let mut b: Vec<f64> = Vec::with_capacity(rows_a);
    let mut h: Vec<f64> = Vec::with_capacity(rows_g);
    let mut buf: Vec<(usize, f64)> = Vec::new();
    for row in constraints {
        let rhs = row.rhs - row.expr.constant;
        buf.clear();
        match row.sense {
            RowSense::Eq => {
                buf.extend(row.expr.coeffs.iter().map(|(&j, &v)| (j, v)));
                if !a.push_row(&buf) {
                    return None;
                }
                b.push(rhs);
            }
            RowSense::Le => {
                buf.extend(row.expr.coeffs.iter().map(|(&j, &v)| (j, v)));
                if !g.push_row(&buf) {
                    return None;
                }
                h.push(rhs);
            }
            RowSense::Ge => {
                buf.extend(row.expr.coeffs.iter().map(|(&j, &v)| (j, -v)));
                if !g.push_row(&buf) {
                    return None;
                }
                h.push(-rhs);
            }
        }
    }
    for (j, v) in variables.iter().enumerate() {
        if v.ub.is_finite() {
            g.push_singleton(j, 1.0);
            h.push(v.ub);
        }
        if v.lb.is_finite() {
            g.push_singleton(j, -1.0);
            h.push(-v.lb);
        }
    }
    Some((a.finish(), b, g.finish(), h))
}

/// [`build_a_g`] の行リスト版 (`csr_from_rows` で構築する従来の実装)。
fn build_a_g_rows(variables: &[VariableData], constraints: &[ConstraintRow]) -> (FaerCsr, Vec<f64>, FaerCsr, Vec<f64>) {
    let n = variables.len();

    let mut a_rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut b: Vec<f64> = Vec::new();
    let mut g_rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut h: Vec<f64> = Vec::new();

    for row in constraints {
        let terms: Vec<(usize, f64)> = row.expr.coeffs.iter().map(|(&j, &v)| (j, v)).collect();
        let rhs = row.rhs - row.expr.constant;
        match row.sense {
            RowSense::Eq => {
                a_rows.push(terms);
                b.push(rhs);
            }
            RowSense::Le => {
                g_rows.push(terms);
                h.push(rhs);
            }
            RowSense::Ge => {
                g_rows.push(terms.into_iter().map(|(j, v)| (j, -v)).collect());
                h.push(-rhs);
            }
        }
    }

    for (j, v) in variables.iter().enumerate() {
        if v.ub.is_finite() {
            g_rows.push(vec![(j, 1.0)]);
            h.push(v.ub);
        }
        if v.lb.is_finite() {
            g_rows.push(vec![(j, -1.0)]);
            h.push(-v.lb);
        }
    }

    let a = csr_from_rows(&a_rows, n);
    let g = csr_from_rows(&g_rows, n);
    (a, b, g, h)
}

/// [`run_extended`] の結果: スケール・縮小後の問題データと復元用情報。
///
/// 全データはスケール後の空間にある。呼び出し側は解を得たら
/// [`postsolve_log`](Self::postsolve_log) を**逆順**に適用して消去変数を復元し、
/// その後 [`scaling::unscale_x`] でスケールを戻す。
///
/// 消去された変数の上下限は `[0, 0]` に固定済み。上下限を G の行として
/// 含めた形が欲しい場合は [`g_h`](Self::g_h) を使う。
pub struct ExtendedPresolveResult {
    /// 適用したスケーリング係数 (解の逆スケールに使う)。
    pub scaling: Scaling,
    /// 縮小後の等式行列 A。
    pub a: FaerCsr,
    /// 縮小後の等式右辺 b。
    pub b: Vec<f64>,
    /// 各変数の下限 (消去変数は 0)。
    pub lb: Vec<f64>,
    /// 各変数の上限 (消去変数は 0)。
    pub ub: Vec<f64>,
    /// G の多変数行 (`<=` 行、上下限行を除く)。
    pub real_rows: Vec<Vec<(usize, f64)>>,
    /// `real_rows` の右辺。
    pub real_rhs: Vec<f64>,
    /// 縮小後の目的係数 c。
    pub c: Vec<f64>,
    /// 前処理で実行不能が証明されたら `true` (他のフィールドは信頼できない)。
    pub infeasible: bool,
    /// [`freevar::eliminate_free_variables`] が非有界な自由変数を見つけたら `true`
    /// (他のフィールドは信頼できない)。`infeasible` とは排他。
    pub unbounded: bool,
    /// 全手法の消去ステップを実行順に記録したログ (通常の代入と平行列の併合)。
    /// 復元時はこのログを**逆順に 1 回**走査する。種類別に分けて別々に戻すと、
    /// 併合前に記録された代入が併合後の値を読んでしまい誤った解になる。
    pub postsolve_log: Vec<PostsolveStep>,
}

impl ExtendedPresolveResult {
    /// 上下限を単一変数行として G に戻した `(G, h)` を必要時に構築して返す
    /// (内点法用。単体法は分離形式を直接読む)。
    pub fn g_h(&self) -> (FaerCsr, Vec<f64>) {
        propagate::rebuild_g_ref(self.lb.len(), &self.real_rows, &self.real_rhs, &self.lb, &self.ub)
    }
}

/// [`ExtendedPresolveResult::postsolve_log`] の 1 ステップ。
/// 2 種類の消去を 1 本の時系列に並べ、逆順 1 パスで戻せるようにする。
pub enum PostsolveStep {
    /// 線形代入 (他変数の値から消去変数を計算する。[`colsingleton::Substitution::value`])。
    Sub(colsingleton::Substitution),
    /// 平行列の併合 (残した列 `kept` の値を読み書きして 2 列に分配する。
    /// [`parallelcols::Substitution::apply`])。
    ParallelCol(parallelcols::Substitution),
}

/// 実行不能と判定したときの [`ExtendedPresolveResult`] を作る (`_n` は未使用)。
fn extended_infeasible(sc: Scaling, a: FaerCsr, b: Vec<f64>, c: Vec<f64>, _n: usize) -> ExtendedPresolveResult {
    ExtendedPresolveResult {
        scaling: sc,
        a,
        b,
        lb: Vec::new(),
        ub: Vec::new(),
        real_rows: Vec::new(),
        real_rhs: Vec::new(),
        c,
        infeasible: true,
        unbounded: false,
        postsolve_log: Vec::new(),
    }
}

/// 非有界と判定したときの [`ExtendedPresolveResult`] を作る (`_n` は未使用)。
fn extended_unbounded(sc: Scaling, a: FaerCsr, b: Vec<f64>, c: Vec<f64>, _n: usize) -> ExtendedPresolveResult {
    ExtendedPresolveResult {
        scaling: sc,
        a,
        b,
        lb: Vec::new(),
        ub: Vec::new(),
        real_rows: Vec::new(),
        real_rhs: Vec::new(),
        c,
        infeasible: false,
        unbounded: true,
        postsolve_log: Vec::new(),
    }
}

/// 前処理パイプライン全体を実行する (単体法・内点法の共通入口)。
///
/// 引数:
/// - `n`: 変数数。`a`/`b`: 等式制約、`g`/`h`: 不等式制約 (上下限行を含む)、`c`: 目的係数。
/// - `ruiz_iters`: Ruiz 均衡化の反復回数。
/// - `prop_passes`: 1 回の上下限伝播で行う伝播パスの上限。
/// - `rounds`: 外側ラウンドの上限 (不動点に達すれば早期終了)。
/// - `inner_rounds`: 各ラウンド内の rowsingleton ↔ colsingleton 反復の上限。
/// - `allow_unbounded_verdict`: `freevar` による「非有界」判定を結果として返してよいか
///   (偽なら非有界を検出しても freevar の段を飛ばし、判定をソルバーに委ねる)。
///
/// 外側ラウンドは「行数と全上下限が前ラウンドから変化しない」まで繰り返す
/// (HiGHS の `HPresolve::run` と同じ形)。`doubleton` は 1 ラウンドでも空振りすると
/// 以降停止する (ラッチ)。
pub fn run_extended(
    n: usize,
    a: &FaerCsr,
    b: &[f64],
    g: &FaerCsr,
    h: &[f64],
    c: &[f64],
    ruiz_iters: usize,
    prop_passes: usize,
    rounds: usize,
    inner_rounds: usize,
    allow_unbounded_verdict: bool,
) -> ExtendedPresolveResult {
    // `ENOMOTO_PROF_PRESOLVE` が設定されていれば各段の所要時間を stderr に出す。
    let profile = env_str!("ENOMOTO_PROF_PRESOLVE").is_some();
    // `$body` を評価し、プロファイル有効時はその所要時間を `$label` 付きで表示する。
    macro_rules! timed_step {
        ($label:expr, $body:expr) => {{
            if profile {
                let __t0 = std::time::Instant::now();
                let __r = $body;
                eprintln!("  PROF_PRESOLVE {:20} {:8.3}ms", $label, __t0.elapsed().as_secs_f64() * 1e3);
                __r
            } else {
                $body
            }
        }};
    }
    // 前処理全体の開始時刻 (プロファイル表示用)。
    let wall_t0 = std::time::Instant::now();

    let sc = timed_step!("scaling::compute", scaling::compute(n, a, g, c, ruiz_iters));
    let (mut a, mut g, mut b, mut h, mut c) = timed_step!("scaling::apply", scaling::apply(&sc, a, g, b, h, c));

    // 等式の冗長行削除をどこで行うか (`ENOMOTO_REDEQ_MODE`):
    //   0 = ここで完全版 (重複 + 階数判定) を実行。ブロック分解の辺フィルタ用に
    //       伝播前の上下限を渡す (緩い上下限でも安全)。
    //   1 = (既定) ここでは重複行の削除のみ、階数判定は行わない。
    //   2 = ここでは重複のみ、ラウンドループ後の縮小した A で階数判定を 1 回行う。
    let redeq_mode = tunable!("ENOMOTO_REDEQ_MODE", REDEQ_MODE, usize);
    let (na, nb) = if redeq_mode == 0 {
        let (pre_lb, pre_ub) = propagate::extract_bounds_only(n, &g, &h);
        timed_step!("reduce_equalities", redundancy::reduce_equalities(&a, &b, n, &pre_lb, &pre_ub))
    } else {
        timed_step!("reduce_equalities(dedupe)", redundancy::dedupe_equalities(&a, &b, n))
    };
    a = na;
    b = nb;
    if profile && env_str!("ENOMOTO_PROF_REDUNDANCY").is_some() {
        use std::sync::atomic::Ordering::Relaxed;
        let total = redundancy::PROF_TOTAL_STEPS.load(Relaxed);
        let trivial = redundancy::PROF_TRIVIAL_STEPS.load(Relaxed);
        eprintln!(
            "  PROF_REDUNDANCY sparse_steps={total} trivial_steps={trivial} ({:.1}%)",
            100.0 * trivial as f64 / total.max(1) as f64
        );
    }
    let (ng, nh) = timed_step!("reduce_inequalities", redundancy::reduce_inequalities(&g, &h, n));
    g = ng;
    h = nh;

    // 注: parallelrows / rowdominance / dominatedcol / stuffing / sparsify / smallcoeff は
    // 実装済みだが接続していない (経緯は改良履歴メモを参照)。

    // ラウンド開始前の上下限を凍結して保持する (`dualpropagate::propagate_dual_bounds` は伝播で
    // 狭める前の元の上下限を必要とする)。同じ抽出結果を最初のラウンドの伝播にも使う。
    let (first_lb, first_ub, first_rows, first_rhs) = propagate::extract_bounds(n, &g, &h);
    let (orig_lb, orig_ub) = (first_lb.clone(), first_ub.clone());
    // 大きな問題向けの設定 (`LARGE_PRESOLVE_MIN_ROWS` 参照): 外側ラウンドの延長と `ineqsingleton` の既定有効化。
    let large = {
        let min_rows = tunable!("ENOMOTO_T_LARGE_PRESOLVE_MIN_ROWS", LARGE_PRESOLVE_MIN_ROWS, usize);
        min_rows != 0 && b.len() + first_rows.len() >= min_rows
    };
    // 不動点まで回すか (`PRESOLVE_FIXPOINT`)。有効なら外側ラウンドの数に固定の上限を置かず、不動点か作業量の予算
    // (`PRESOLVE_WORK_BUDGET` × 問題の大きさ) の消費で打ち切る。無効なら従来の `rounds` ラウンド (+ 大きな問題の延長)。
    let fixpoint_mode = tunable!("ENOMOTO_T_PRESOLVE_FIXPOINT", PRESOLVE_FIXPOINT, usize) != 0;
    // 1 回の上下限伝播も有意な変化がなくなるまで回すか (`PROPAGATION_FIXPOINT`、既定は無効で `prop_passes` パスごとに
    // 他の段と交互に回す)。
    let prop_fixpoint = fixpoint_mode && tunable!("ENOMOTO_T_PROPAGATION_FIXPOINT", PROPAGATION_FIXPOINT, usize) != 0;
    // 作業量の予算 (読んだ非零の延べ数)。問題の大きさ (等式行と多変数の不等式行の非零数 + 列数 + 行数) に比例させる。
    let work_budget = if fixpoint_mode {
        let size = a.compute_nnz() + first_rows.iter().map(|r| r.len()).sum::<usize>() + n + b.len() + first_rows.len();
        (tunable!("ENOMOTO_T_PRESOLVE_WORK_BUDGET", PRESOLVE_WORK_BUDGET, f64) * size as f64) as usize
    } else {
        usize::MAX
    };
    // これまでに使った作業量 (伝播で読んだ非零 + 各ラウンドの他の段の分として、ラウンド終了時の非零数 + 列数)。
    let mut work_used = 0usize;
    // 上下限伝播 (不等式・等式) のパスの打ち切り条件。既定は変化がある限り `prop_passes` パスまで。
    // `prop_fixpoint` なら最初の `prop_passes` パスは同じく変化がある限り続け、その先は有意な変化
    // (`FIXPOINT_RELTOL`、外側ループの不動点判定と同じ基準) がある限り、作業量の予算の残りまで続ける。
    let pass_limit = |prop_passes: usize, work_used: usize| -> propagate::PassLimit {
        if prop_fixpoint {
            propagate::PassLimit {
                min_passes: prop_passes,
                max_passes: usize::MAX,
                sig_reltol: tunable!("ENOMOTO_T_FIXPOINT_RELTOL", FIXPOINT_RELTOL, f64),
                max_work: work_budget.saturating_sub(work_used),
            }
        } else {
            propagate::PassLimit::fixed(prop_passes)
        }
    };
    // 延長ラウンドの数 (`PRESOLVE_EXTRA_ROUNDS_LARGE`、従来モードのみ)。大きな問題で `rounds` 回のラウンドを終えても
    // 不動点に達していなければ、伝播パス数を `PROPAGATION_PASSES_LARGE` に上げてこの回数まで続ける。
    let extra_rounds = if large && !fixpoint_mode { tunable!("ENOMOTO_T_PRESOLVE_EXTRA_ROUNDS_LARGE", PRESOLVE_EXTRA_ROUNDS_LARGE, usize) } else { 0 };
    // 外側ラウンド数の上限 (不動点モードでは作業量の予算だけで打ち切る)。
    let max_rounds = if fixpoint_mode { usize::MAX } else { rounds.max(1) + extra_rounds };
    let mut prop_passes = prop_passes;
    // 延長の判定用 (大きな問題のみ): 直前のラウンド終了時の構造 (A の行数, G の多変数行数, 固定列数, ログ長) と、
    // 最後のラウンドで構造が変わったか。上下限だけが少しずつ締まり続ける問題 (neos) は延長しない。
    let mut ext_prev_struct: Option<(usize, usize, usize, usize)> = None;
    let mut ext_last_round_structural = false;
    let ineq_singleton_on = match env_str!("ENOMOTO_INEQ_SINGLETON") {
        Some(v) => v != "0",
        None => large && tunable!("ENOMOTO_T_INEQ_SINGLETON_LARGE", INEQ_SINGLETON_LARGE, usize) != 0,
    };

    // 全消去ステップの時系列ログ。
    let mut postsolve_log: Vec<PostsolveStep> = Vec::new();

    // doubleton を続けて実行するか (空振りが `DOUBLETON_STRIKES` 回続くと以降停止する片道ラッチ)。
    let mut doubleton_active = true;
    // doubleton の連続空振り回数。
    let mut doubleton_empty_streak = 0usize;

    // dualpropagate 用の同様のラッチ (含意等式も固定列も見つからない回が続くと停止)。
    let mut dualpropagate_active = true;
    // dualpropagate の連続空振り回数。
    let mut dualpropagate_empty_streak = 0usize;

    // parallelcols 用のラッチ。最初のラウンドは空振りでも後のラウンドで効く問題があるため、
    // 既定では 2 回連続の空振りで停止する (`PARALLELCOLS_STRIKES`)。
    let mut parallelcols_active = true;
    // parallelcols の連続空振り回数。
    let mut parallelcols_empty_streak = 0usize;

    // 不動点判定用: 前ラウンド終了時の (A の行数, G の行数, lb, ub)。
    // 変化がなければ以降のラウンドも何も見つけないので打ち切る。
    let mut prev_signature: Option<(usize, usize, Vec<f64>, Vec<f64>)> = None;
    // `ROUND_STRUCT_STOP` 用: 前ラウンドの (A の行数, G の多変数行数, 固定列数, ログ長)。
    let mut prev_struct: Option<(usize, usize, usize, usize)> = None;
    // 等式行伝播が何も見つけなくなったか (`EQPROP_SKIP_IDLE` 有効時のみ使用)。
    let mut eqprop_idle = false;
    // G を上下限と多変数行に分離したまま保持するか。G が
    // `rebuild_g_ref(cur_real_rows, cur_real_rhs, lb, ub)` と正確に一致する場合は CSR を作らず
    // `g_split` を立て、G の読み手は分離形式 (`GView::Split` など) を使う。
    // ラウンド間では分離形式を `carry` で受け渡す。
    let split_enabled = tunable!("ENOMOTO_T_PRESOLVE_SPLIT_G", PRESOLVE_SPLIT_G, usize) != 0;
    // 次ラウンドの伝播に渡す分離形式 G (多変数行, 右辺, lb, ub)。最初はループ前の抽出結果。
    let mut carry: Option<(Vec<Vec<(usize, f64)>>, Vec<f64>, Vec<f64>, Vec<f64>)> = Some((first_rows, first_rhs, first_lb, first_ub));
    // `ENOMOTO_DEBUG_PRESOLVE_ROUNDS`: 外側ラウンドごとの縮約と伝播のパス数、打ち切り理由を stderr に出す。
    let debug_rounds = env_str!("ENOMOTO_DEBUG_PRESOLVE_ROUNDS").is_some();
    // 外側ループを抜けた理由 (表示用)。
    let mut stop_reason = "cap";
    // 実行した外側ラウンド数 (表示用)。
    let mut rounds_done = 0usize;
    // 外側ラウンドループ。
    for round_idx in 0..max_rounds {
        if work_used >= work_budget {
            stop_reason = "budget";
            break;
        }
        rounds_done = round_idx + 1;
        if !fixpoint_mode && round_idx == rounds.max(1) {
            // 通常のラウンド数を使い切っても不動点に達していない (大きな問題のみここに来る)。最後のラウンドでも
            // 行・列の縮約が進んでいれば、上下限伝播が 1 ラウンドに数段しか進まない連鎖が残っているので、パス数を
            // 上げて延長する。上下限の変化だけなら従来どおりここで止める。
            if !ext_last_round_structural {
                stop_reason = "cap(bounds-only)";
                break;
            }
            prop_passes = prop_passes.max(tunable!("ENOMOTO_T_PROPAGATION_PASSES_LARGE", PROPAGATION_PASSES_LARGE, usize));
        }
        let prop = timed_step!(
            "propagate",
            match carry.take() {
                Some((rows, rhs, clb, cub)) => propagate::propagate_split(n, clb, cub, rows, rhs, pass_limit(prop_passes, work_used)),
                None => propagate::propagate_without_g_rebuild(n, &g, &h, pass_limit(prop_passes, work_used)),
            }
        );
        if prop.infeasible {
            return extended_infeasible(sc, a, b, c, n);
        }
        work_used = work_used.saturating_add(prop.work);
        // 表示用: この回の伝播のパス数と、変化のないパスで止まったか。
        let (dbg_prop_passes, dbg_prop_converged) = (prop.passes_used, prop.converged);
        // 表示用: 等式行伝播が変化のないパスで止まったか (実行しなければ `None`)。
        let mut dbg_eq_converged: Option<bool> = None;
        let mut lb = prop.lb;
        let mut ub = prop.ub;
        // 現在の G の多変数行とその右辺。代入による行の書き換えを反映して随時更新する。
        let mut cur_real_rows = prop.real_rows;
        let mut cur_real_rhs = prop.real_rhs;
        // `true` の間、G は分離形式 `(cur_real_rows, cur_real_rhs, lb, ub)` でのみ保持され、
        // 変数 `g`/`h` は古い。
        let mut g_split = false;
        // `G := rebuild_g_ref(cur_real_rows, cur_real_rhs, lb, ub)` を設定する。
        // 分離形式で正確に表せる場合は CSR を作らず `g_split = true` にする。
        macro_rules! set_g_from_split {
            ($label:expr) => {
                if split_enabled && propagate::split_is_canonical(n, &cur_real_rows, &lb, &ub) {
                    g_split = true;
                } else {
                    let (ng, nh) = timed_step!($label, propagate::rebuild_g_ref(n, &cur_real_rows, &cur_real_rhs, &lb, &ub));
                    g = ng;
                    h = nh;
                    g_split = false;
                }
            };
        }
        // 現在の G を読むためのビュー (分離形式か行列形式か)。
        macro_rules! g_view {
            () => {
                if g_split {
                    propagate::GView::Split { rows: &cur_real_rows, rhs: &cur_real_rhs, lb: &lb, ub: &ub }
                } else {
                    propagate::GView::Mat { g: &g, h: &h }
                }
            };
        }

        // 等式行による上下限伝播 (`propagate::propagate_equalities`)。最初の
        // `EQPROP_ROUNDS` ラウンドだけ実行する。`EQPROP_SKIP_IDLE` が有効なら、
        // 何も見つからなかった時点で以降のラウンドを省略する (結果が変わりうる)。
        if round_idx < tunable!("ENOMOTO_T_EQPROP_ROUNDS", EQPROP_ROUNDS, usize) && !eqprop_idle {
            // 等式行伝播は不動点モードでも従来どおり `prop_passes` パスまで (`EQPROP_FIXPOINT` 参照)。
            let eq_limit = if tunable!("ENOMOTO_T_EQPROP_FIXPOINT", EQPROP_FIXPOINT, usize) != 0 { pass_limit(prop_passes, work_used) } else { propagate::PassLimit::fixed(prop_passes) };
            let eq = timed_step!("eqprop", propagate::propagate_equalities(&a, &b, &mut lb, &mut ub, eq_limit));
            if eq.infeasible {
                return extended_infeasible(sc, a, b, c, n);
            }
            work_used = work_used.saturating_add(eq.work);
            dbg_eq_converged = Some(eq.converged);
            if tunable!("ENOMOTO_T_EQPROP_SKIP_IDLE", EQPROP_SKIP_IDLE, usize) != 0 && eq.forcing_rows == 0 && eq.fixed_cols == 0 && eq.tightened == 0 {
                eqprop_idle = true;
            }
        }

        // 符号を反転しただけで右辺が釣り合う不等式行の組 (等式を 2 本の `<=` で表したもの) を等式 1 本にして A へ移す
        // (不動点モードのみ、`OPPOSITE_PAIR_EQUALITY`)。
        if fixpoint_mode && tunable!("ENOMOTO_T_OPPOSITE_PAIR_EQUALITY", OPPOSITE_PAIR_EQUALITY, usize) != 0 {
            let pairs = timed_step!("oppositepairs", redundancy::find_opposite_equality_pairs(&cur_real_rows, &cur_real_rhs, crate::params::presolve::PROPAGATE_EPS));
            if debug_rounds && !pairs.is_empty() {
                eprintln!("PRESOLVE_OPPOSITE_PAIRS round={} pairs={}", round_idx, pairs.len());
            }
            if !pairs.is_empty() {
                let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(&a);
                // G から取り除く多変数行の印 (等式にした行とその相方)。
                let mut drop = vec![false; cur_real_rows.len()];
                for &(p, q) in &pairs {
                    a_rows.push(cur_real_rows[p].clone());
                    b.push(cur_real_rhs[p]);
                    drop[p] = true;
                    drop[q] = true;
                }
                a = csr_from_rows(&a_rows, n);
                let mut kept_rows = Vec::with_capacity(cur_real_rows.len());
                let mut kept_rhs = Vec::with_capacity(cur_real_rhs.len());
                for (i, (row, rhs)) in cur_real_rows.into_iter().zip(cur_real_rhs.into_iter()).enumerate() {
                    if !drop[i] {
                        kept_rows.push(row);
                        kept_rhs.push(rhs);
                    }
                }
                cur_real_rows = kept_rows;
                cur_real_rhs = kept_rhs;
            }
        }

        // 双対による固定: 目的係数の向きを妨げる行がない変数を上下限に固定する。
        let fixes = timed_step!("dualfix", dualfix::fix_by_lock_count(n, &a, &cur_real_rows, &c, &lb, &ub));
        for &(j, value) in &fixes {
            lb[j] = value;
            ub[j] = value;
        }

        // 双対実行可能性の伝播による 2 つの縮小 (`dualpropagate`):
        // 全最適解で等号成立する不等式行を等式系へ昇格し、被約費用の符号が確定する列を固定する。
        if dualpropagate_active {
            let dual_red = timed_step!("dualpropagate", dualpropagate::propagate_dual_bounds(n, &a, &cur_real_rows, &c, &lb, &ub, &orig_lb, &orig_ub, prop_passes));
            if dual_red.implied_equalities.is_empty() && dual_red.fixed_columns.is_empty() {
                dualpropagate_empty_streak += 1;
                if dualpropagate_empty_streak >= tunable!("ENOMOTO_T_DUALPROPAGATE_STRIKES", DUALPROPAGATE_STRIKES, usize) {
                    dualpropagate_active = false;
                }
            } else {
                dualpropagate_empty_streak = 0;
                if env_str!("ENOMOTO_DEBUG_DUALPROPAGATE").is_some() {
                    eprintln!("DEBUG_DUALPROPAGATE: implied_equalities={} fixed_columns={}", dual_red.implied_equalities.len(), dual_red.fixed_columns.len());
                }
                for &(j, value) in &dual_red.fixed_columns {
                    lb[j] = value;
                    ub[j] = value;
                }
                if !dual_red.implied_equalities.is_empty() {
                    // 含意等式となった G の多変数行を A へ移し、G から取り除く。
                    let implied = dual_red.implied_equalities;
                    let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(&a);
                    let mut new_b = b.clone();
                    // A へ昇格した G の多変数行の印。
                    let mut promoted = vec![false; cur_real_rows.len()];
                    for &gi in &implied {
                        promoted[gi] = true;
                        a_rows.push(cur_real_rows[gi].clone());
                        new_b.push(cur_real_rhs[gi]);
                    }
                    a = csr_from_rows(&a_rows, n);
                    b = new_b;
                    let mut kept_rows = Vec::with_capacity(cur_real_rows.len() - implied.len());
                    let mut kept_rhs = Vec::with_capacity(cur_real_rhs.len() - implied.len());
                    for (i, (row, rhs)) in cur_real_rows.into_iter().zip(cur_real_rhs.into_iter()).enumerate() {
                        if !promoted[i] {
                            kept_rows.push(row);
                            kept_rhs.push(rhs);
                        }
                    }
                    cur_real_rows = kept_rows;
                    cur_real_rhs = kept_rhs;
                }
            }
        }

        // 不等式行の列シングルトン (`ineqsingleton`、大きな問題か `ENOMOTO_INEQ_SINGLETON` 設定時):
        // 列を上下限に固定するか、その行を等式に変えて後段の colsingleton に消去させる。
        if ineq_singleton_on {
            let isr = timed_step!("ineqsingleton", ineqsingleton::resolve_inequality_singletons(n, &a, &cur_real_rows, &cur_real_rhs, &c, &lb, &ub));
            if env_str!("ENOMOTO_DEBUG_INEQ_SINGLETON").is_some() {
                eprintln!("DEBUG_INEQ_SINGLETON: fixes={} implied_equalities={}", isr.fixes.len(), isr.implied_equalities.len());
            }
            for &(j, value) in &isr.fixes {
                lb[j] = value;
                ub[j] = value;
            }
            if !isr.implied_equalities.is_empty() {
                let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(&a);
                // G から取り除く多変数行の印 (等式化した行と、対になる行)。
                let mut drop = vec![false; cur_real_rows.len()];
                for &(gi, other) in &isr.implied_equalities {
                    a_rows.push(cur_real_rows[gi].clone());
                    b.push(cur_real_rhs[gi]);
                    drop[gi] = true;
                    if let Some(o) = other {
                        drop[o] = true;
                    }
                }
                a = csr_from_rows(&a_rows, n);
                let mut kept_rows = Vec::with_capacity(cur_real_rows.len());
                let mut kept_rhs = Vec::with_capacity(cur_real_rhs.len());
                for (i, (row, rhs)) in cur_real_rows.into_iter().zip(cur_real_rhs.into_iter()).enumerate() {
                    if !drop[i] {
                        kept_rows.push(row);
                        kept_rhs.push(rhs);
                    }
                }
                cur_real_rows = kept_rows;
                cur_real_rhs = kept_rhs;
            }
        }

        // 固定済みの列 (lb == ub) を A と G の多変数行から取り除き、右辺へ移す (`foldfixed`)。
        // 行の項数を減らすことで rowsingleton/doubleton/aggregator の候補が見えるようにする。
        // 何も変わらない場合 (空行なし・零要素なし・固定列なし) は省略する (結果は同一)。
        let is_fixed = |j: usize| lb[j] == ub[j];
        // A に対して foldfixed が何もしないか。
        let a_noop = crate::sparse::csr_is_canonical(&a) && {
            let ar = a.as_ref();
            (0..ar.nrows()).all(|i| !ar.col_indices_of_row_raw(i).is_empty()) && ar.col_indices().iter().all(|&j| !is_fixed(j))
        };
        if !a_noop {
            let a_rows: Vec<Vec<(usize, f64)>> = csr_rows(&a);
            let fold_a = timed_step!("foldfixed(A)", foldfixed::fold_fixed_columns(&a_rows, &b, &lb, &ub, RowSense::Eq));
            if fold_a.infeasible {
                return extended_infeasible(sc, a, b, c, n);
            }
            a = csr_from_rows(&fold_a.rows, n);
            b = fold_a.rhs;
        }

        // G の多変数行に対して foldfixed が何もしないか。
        let g_noop = cur_real_rows.iter().all(|r| !r.is_empty() && r.iter().all(|&(j, v)| v != 0.0 && !is_fixed(j)));
        if !g_noop {
            let fold_g = timed_step!("foldfixed(G)", foldfixed::fold_fixed_columns(&cur_real_rows, &cur_real_rhs, &lb, &ub, RowSense::Le));
            if fold_g.infeasible {
                return extended_infeasible(sc, a, b, c, n);
            }
            cur_real_rows = fold_g.rows;
            cur_real_rhs = fold_g.rhs;
        }

        // 内側ループ: rowsingleton → (最初のパスのみ doubleton) → colsingleton を最大
        // `inner_rounds` 回繰り返す。一方の消去が他方の新しい候補を生むため。
        // (A の行数, G の行数) が変わらなくなったら終了する。
        // 前回の内側パス終了時の (A の行数, G の行数)。
        let mut inner_prev_signature: Option<(usize, usize)> = None;
        for inner_idx in 0..inner_rounds.max(1) {
            // 行シングルトン: 1 変数だけの等式行からその変数を固定する。
            let rs = timed_step!("rowsingleton", rowsingleton::fix_singleton_equalities(n, &a, &b, &lb, &ub));
            if rs.infeasible {
                return extended_infeasible(sc, a, b, c, n);
            }
            for &(j, value) in &rs.fixes {
                lb[j] = value;
                ub[j] = value;
            }
            a = rs.a;
            b = rs.b;

            set_g_from_split!("rebuild_g(inner)");
            // `(g, h)` がまだ `rebuild_g_ref(cur_real_rows, cur_real_rhs, lb, ub)` そのものか
            // (下の `extract_bounds(inner)` を省略できるかの判定に使う)。
            let mut g_is_rebuilt = true;

            if inner_idx == 0 && doubleton_active {
                // doubleton 等式の代入消去。`None` は何も変化なし。
                let dbl = timed_step!("doubleton", doubleton::eliminate_doubleton_equalities_view(n, &a, &b, g_view!(), &c));
                if dbl.as_ref().is_none_or(|d| d.substitutions.is_empty()) {
                    doubleton_empty_streak += 1;
                    if doubleton_empty_streak >= tunable!("ENOMOTO_T_DOUBLETON_STRIKES", DOUBLETON_STRIKES, usize) {
                        doubleton_active = false;
                    }
                } else {
                    doubleton_empty_streak = 0;
                }
                if let Some(dbl) = dbl {
                    g_is_rebuilt = false;
                    g_split = false;
                    a = dbl.a;
                    b = dbl.b;
                    g = dbl.g;
                    h = dbl.h;
                    c = dbl.c;
                    // 消去した変数の上下限を直ちに `[0, 0]` に固定する (次の内側パスの
                    // G 再構築で元の上下限が箱制約行として復活しないように)。
                    for sub in &dbl.substitutions {
                        lb[sub.var] = 0.0;
                        ub[sub.var] = 0.0;
                    }
                    postsolve_log.extend(dbl.substitutions.into_iter().map(PostsolveStep::Sub));
                }
            }

            // 列シングルトン等式の代入消去。
            let cs = timed_step!("colsingleton", colsingleton::eliminate_singleton_equalities_view(n, &a, &b, g_view!(), &c));
            // colsingleton が何も消去しなかったか。
            let cs_unchanged = cs.substitutions.is_empty();
            a = cs.a;
            b = cs.b;
            c = cs.c;
            if !cs.substitutions.is_empty() {
                // 消去した変数の (もう無意味な) 箱制約行を G から除いてから `cs.extra_g_rows` を
                // 追加する。残すと消去変数が独立した自由度を持つ「幽霊列」として後段に見えてしまう。
                // このパスで消去された変数の印。
                let mut eliminated_this_pass = vec![false; n];
                for s in &cs.substitutions {
                    eliminated_this_pass[s.var] = true;
                }
                // 新しい G = 現在の G から消去変数の単一要素行を除いたもの + `cs.extra_g_rows`。
                // CsrRowBuilder に直接組み立てる (分離形式なら多変数行 → 消去されていない列の
                // 上下限行の順、上下限は下の固定より前の値)。行が拒否された場合
                // (列の重複) は行リストを明示的に作って構築し直す。
                let (ng, nh) = {
                    // 新しい G の行数の上限 (容量確保用)。
                    let n_rows = if g_split { cur_real_rows.len() + 2 * n } else { g.nrows() } + cs.extra_g_rows.len();
                    let mut builder = crate::sparse::CsrRowBuilder::with_capacity(n, n_rows, 0);
                    let mut h_vec: Vec<f64> = Vec::with_capacity(n_rows);
                    // 全行がビルダーに受け付けられたか。
                    let mut ok = true;
                    if g_split {
                        for (row, &r) in cur_real_rows.iter().zip(&cur_real_rhs) {
                            ok = ok && builder.push_row(row);
                            h_vec.push(r);
                        }
                        for j in 0..n {
                            if eliminated_this_pass[j] {
                                continue;
                            }
                            if ub[j].is_finite() {
                                builder.push_singleton(j, 1.0);
                                h_vec.push(ub[j]);
                            }
                            if lb[j].is_finite() {
                                builder.push_singleton(j, -1.0);
                                h_vec.push(-lb[j]);
                            }
                        }
                    } else {
                        let gr = g.as_ref();
                        let mut buf: Vec<(usize, f64)> = Vec::new();
                        for i in 0..gr.nrows() {
                            let cols = gr.col_indices_of_row_raw(i);
                            if cols.len() == 1 && eliminated_this_pass[cols[0]] {
                                continue;
                            }
                            buf.clear();
                            buf.extend(cols.iter().copied().zip(gr.values_of_row(i).iter().copied()));
                            ok = ok && builder.push_row(&buf);
                            h_vec.push(h[i]);
                        }
                    }
                    for row in &cs.extra_g_rows {
                        ok = ok && builder.push_row(row);
                    }
                    h_vec.extend_from_slice(&cs.extra_h);
                    if ok {
                        (builder.finish(), h_vec)
                    } else {
                        let (g0, h0) = if g_split { propagate::rebuild_g_ref(n, &cur_real_rows, &cur_real_rhs, &lb, &ub) } else { (g.clone(), h.clone()) };
                        let gr = g0.as_ref();
                        let mut g_rows: Vec<Vec<(usize, f64)>> = Vec::new();
                        let mut h_vec: Vec<f64> = Vec::new();
                        for i in 0..gr.nrows() {
                            let row: Vec<(usize, f64)> = csr_row_vec(&g0, i);
                            if row.len() == 1 && eliminated_this_pass[row[0].0] {
                                continue;
                            }
                            g_rows.push(row);
                            h_vec.push(h0[i]);
                        }
                        g_rows.extend(cs.extra_g_rows.iter().cloned());
                        h_vec.extend_from_slice(&cs.extra_h);
                        (csr_from_rows(&g_rows, n), h_vec)
                    }
                };
                g = ng;
                h = nh;
                g_split = false;
                // doubleton と同じ理由で、消去変数の上下限を直ちに `[0, 0]` に固定する。
                for (j, &e) in eliminated_this_pass.iter().enumerate() {
                    if e {
                        lb[j] = 0.0;
                        ub[j] = 0.0;
                    }
                }
            }
            postsolve_log.extend(cs.substitutions.into_iter().map(PostsolveStep::Sub));

            // 内側ループの不動点判定。
            let inner_signature = (a.nrows(), g_view!().nrows());
            if inner_prev_signature == Some(inner_signature) {
                break;
            }
            inner_prev_signature = Some(inner_signature);

            // 更新した G を再び上下限と多変数行に分解し、次の内側パスに渡す
            // (代入で 1 変数に縮んだ行は上下限として取り込み、より厳しい方を採る)。
            // G が今回の再構築結果のままで多変数行も正準形なら、往復しても同一なので省略する。
            // 分解しても現在の値がそのまま返るだけか。
            let round_trip = g_split || g_is_rebuilt && cs_unchanged && cur_real_rows.iter().all(|r| r.len() != 1 && r.iter().all(|&(_, v)| v != 0.0) && r.windows(2).all(|w| w[0].0 < w[1].0));
            if !round_trip {
                let (refreshed_lb, refreshed_ub, refreshed_real_rows, refreshed_real_rhs) = timed_step!("extract_bounds(inner)", propagate::extract_bounds(n, &g, &h));
                for j in 0..n {
                    lb[j] = lb[j].max(refreshed_lb[j]);
                    ub[j] = ub[j].min(refreshed_ub[j]);
                }
                cur_real_rows = refreshed_real_rows;
                cur_real_rhs = refreshed_real_rhs;
            }
        }

        // Aggregator (HiGHS の `HPresolve::aggregator`): 等式行から implied-free と
        // 分かる列を代入消去する (境界保存行は不要)。HiGHS と同様、高速な
        // singleton/doubleton ループの後に置く。
        // 実装の切り替え: `ENOMOTO_DISABLE_AGGREGATOR` で無効、`ENOMOTO_XROW_AGGREGATOR` で
        // 行横断版、`ENOMOTO_ROWLOCAL_AGGREGATOR` で旧行局所版、既定は v2。
        let agg = if env_str!("ENOMOTO_DISABLE_AGGREGATOR").is_some() {
            None
        } else if env_str!("ENOMOTO_XROW_AGGREGATOR").is_some() {
            Some(timed_step!("aggregator", aggregator::eliminate_implied_free_columns_xrow(n, &a, &b, &c, &lb, &ub, &cur_real_rows, &cur_real_rhs)))
        } else if env_str!("ENOMOTO_ROWLOCAL_AGGREGATOR").is_some() {
            // `None` = 何も消去されなかった (問題は不変)。
            timed_step!("aggregator", aggregator::eliminate_implied_free_columns_if_any(n, &a, &b, &c, &lb, &ub, &cur_real_rows, &cur_real_rhs))
        } else {
            // 既定の v2 実装。`None` = 候補なし (問題をコピーせずに判定)。
            timed_step!("aggregator", aggregator::eliminate_implied_free_columns_v2_if_any(n, &a, &b, &c, &lb, &ub, &cur_real_rows, &cur_real_rhs, aggregator::AggOptions::from_env()))
        };
        if let Some(agg) = agg {
            if env_str!("ENOMOTO_DEBUG_AGGREGATOR").is_some() {
                eprintln!("DEBUG_AGGREGATOR: eliminated={}", agg.substitutions.len());
            }
            if !agg.substitutions.is_empty() {
                a = agg.a;
                b = agg.b;
                c = agg.c;
                for sub in &agg.substitutions {
                    lb[sub.var] = 0.0;
                    ub[sub.var] = 0.0;
                }
                postsolve_log.extend(agg.substitutions.into_iter().map(PostsolveStep::Sub));
                cur_real_rows = agg.real_rows;
                cur_real_rhs = agg.real_rhs;
                // 後段の重複削除と次ラウンドの伝播が今回の変更を見るよう G を再設定する。
                set_g_from_split!("rebuild_g(agg)");
            }
        }

        // 平行列の併合 (`parallelcols`)。aggregator の直後に置く (HiGHS と同様)。
        // `PARALLELCOLS_STRIKES` 回連続で空振りすると以降停止する。
        // `ENOMOTO_DISABLE_PARALLELCOLS` で無効化。
        let pc = if parallelcols_active && env_str!("ENOMOTO_DISABLE_PARALLELCOLS").is_none() {
            // 内側の `None` = 実行したが併合なし (問題のコピーは作らない)。
            Some(timed_step!("parallelcols", parallelcols::merge_parallel_columns_if_any(n, &a, &cur_real_rows, &c, &lb, &ub)))
        } else {
            None
        };
        if let Some(pc) = pc {
            if env_str!("ENOMOTO_DEBUG_PARALLELCOLS").is_some() {
                eprintln!("DEBUG_PARALLELCOLS: eliminated={}", pc.as_ref().map_or(0, |pc| pc.substitutions.len()));
            }
            if let Some(pc) = pc {
                parallelcols_empty_streak = 0;
                a = pc.a;
                c = pc.c;
                lb = pc.lb;
                ub = pc.ub;
                cur_real_rows = pc.real_rows;
                postsolve_log.extend(pc.substitutions.into_iter().map(PostsolveStep::ParallelCol));
                // aggregator と同じ理由で G を再設定する。
                set_g_from_split!("rebuild_g(pc)");
            } else {
                parallelcols_empty_streak += 1;
                if parallelcols_empty_streak >= tunable!("ENOMOTO_T_PARALLELCOLS_STRIKES", PARALLELCOLS_STRIKES, usize) {
                    parallelcols_active = false;
                }
            }
        }

        // 不等式行の重複削除 (ハッシュによる O(nnz) の安価な処理) を毎ラウンド再実行する。
        // 代入で行が書き換わり、元は異なる 2 行が重複になることがあるため。
        if g_split {
            if let Some(keep) = timed_step!("reduce_inequalities(round)", redundancy::reduce_inequality_rows(&cur_real_rows, &cur_real_rhs)) {
                // `keep[i]` が真の行だけを残す。
                let mut k = keep.iter();
                cur_real_rows.retain(|_| *k.next().unwrap());
                let mut k = keep.iter();
                cur_real_rhs.retain(|_| *k.next().unwrap());
            }
        } else {
            let (rg, rh) = timed_step!("reduce_inequalities(round)", redundancy::reduce_inequalities(&g, &h, n));
            g = rg;
            h = rh;
        }

        // 構造 (A の行数, G の多変数行数, 固定列数, ログ長) が前ラウンドと同じなら、上下限の値の変化を無視して
        // 打ち切る (不動点モードでは既定で有効 `ROUND_STRUCT_STOP_FIXPOINT`、従来モードでは `ROUND_STRUCT_STOP`)。
        let struct_stop_default = if fixpoint_mode { ROUND_STRUCT_STOP_FIXPOINT } else { ROUND_STRUCT_STOP };
        let struct_stop = if tunable!("ENOMOTO_T_ROUND_STRUCT_STOP", struct_stop_default, usize) != 0 {
            let g_multi = if g_split {
                cur_real_rows.len()
            } else {
                let gr = g.as_ref();
                (0..gr.nrows()).filter(|&i| gr.col_indices_of_row_raw(i).len() > 1).count()
            };
            let fixed = (0..n).filter(|&j| lb[j] == ub[j]).count();
            let st = (a.nrows(), g_multi, fixed, postsolve_log.len());
            let stop = prev_struct == Some(st);
            prev_struct = Some(st);
            stop
        } else {
            false
        };

        // 外側ループの不動点判定用の状態。分離形式なら次ラウンドへ `carry` で渡す。
        if extra_rounds > 0 {
            let g_multi = if g_split {
                cur_real_rows.len()
            } else {
                let gr = g.as_ref();
                (0..gr.nrows()).filter(|&i| gr.col_indices_of_row_raw(i).len() > 1).count()
            };
            let st = (a.nrows(), g_multi, (0..n).filter(|&j| lb[j] == ub[j]).count(), postsolve_log.len());
            ext_last_round_structural = ext_prev_struct.is_some_and(|p| p != st);
            ext_prev_struct = Some(st);
        }
        // このラウンドの伝播以外の段の作業量 (各段はほぼ非零数に比例する)。
        let round_nnz = a.compute_nnz() + if g_split { cur_real_rows.iter().map(|r| r.len()).sum::<usize>() } else { g.compute_nnz() } + n;
        work_used = work_used.saturating_add(round_nnz);
        if debug_rounds {
            let g_multi = if g_split {
                cur_real_rows.len()
            } else {
                let gr = g.as_ref();
                (0..gr.nrows()).filter(|&i| gr.col_indices_of_row_raw(i).len() > 1).count()
            };
            let fixed = (0..n).filter(|&j| lb[j] == ub[j]).count();
            eprintln!(
                "PRESOLVE_ROUND {} a_rows={} g_multi={} fixed={} log={} prop_passes={} prop_conv={} eq_conv={:?} work={} budget={} t={:.1}ms",
                round_idx,
                a.nrows(),
                g_multi,
                fixed,
                postsolve_log.len(),
                dbg_prop_passes,
                dbg_prop_converged,
                dbg_eq_converged,
                work_used,
                work_budget,
                wall_t0.elapsed().as_secs_f64() * 1e3
            );
        }
        let signature = (a.nrows(), g_view!().nrows(), lb.clone(), ub.clone());
        if g_split {
            carry = Some((cur_real_rows, cur_real_rhs, lb, ub));
        }
        if struct_stop {
            stop_reason = "struct_stop";
            break;
        }
        // 相対 `FIXPOINT_RELTOL` 未満の上下限変化は進展とみなさない (巡回的な行構造で
        // 伝播が上下限を等比級数的に削り続けてもラウンドを使い切らないため。強化した
        // 上下限自体は保持する)。`ENOMOTO_FIXPOINT_EXACT` で厳密比較に戻す。
        let same = |p: &(usize, usize, Vec<f64>, Vec<f64>)| {
            if env_str!("ENOMOTO_FIXPOINT_EXACT").is_some() {
                return *p == signature;
            }
            let close = |x: &[f64], y: &[f64]| x.iter().zip(y).all(|(&u, &v)| u == v || (u - v).abs() <= tunable!("ENOMOTO_T_FIXPOINT_RELTOL", FIXPOINT_RELTOL, f64) * (1.0 + u.abs().max(v.abs())));
            p.0 == signature.0 && p.1 == signature.1 && close(&p.2, &signature.2) && close(&p.3, &signature.3)
        };
        if prev_signature.as_ref().is_some_and(same) {
            stop_reason = "fixpoint";
            break;
        }
        prev_signature = Some(signature);
    }

    if redeq_mode == 2 && a.nrows() > 0 {
        // 遅延した階数判定 (`REDEQ_MODE == 2`): 縮小済みの A に対して一次従属行を削除する。
        // 現在の上下限はブロック分解の辺フィルタにだけ使う。
        let (cur_lb, cur_ub) = match &carry {
            Some((_, _, clb, cub)) => (clb.clone(), cub.clone()),
            None => propagate::extract_bounds_only(n, &g, &h),
        };
        let (na, nb) = timed_step!("reduce_equalities(post)", redundancy::reduce_equalities(&a, &b, n, &cur_lb, &cur_ub));
        a = na;
        b = nb;
    }

    // 最終の上下限伝播。
    let prop = timed_step!(
        "final propagate",
        match carry.take() {
            Some((rows, rhs, clb, cub)) => propagate::propagate_split(n, clb, cub, rows, rhs, pass_limit(prop_passes, work_used)),
            None => propagate::propagate_without_g_rebuild(n, &g, &h, pass_limit(prop_passes, work_used)),
        }
    );
    if profile {
        eprintln!("PROF_PRESOLVE total {:.3}ms", wall_t0.elapsed().as_secs_f64() * 1e3);
    }
    if debug_rounds {
        eprintln!(
            "PRESOLVE_ROUNDS_END rounds={} reason={} final_prop_passes={} final_prop_conv={} t={:.1}ms",
            rounds_done,
            stop_reason,
            prop.passes_used,
            prop.converged,
            wall_t0.elapsed().as_secs_f64() * 1e3
        );
    }
    if prop.infeasible {
        return ExtendedPresolveResult {
            scaling: sc,
            a,
            b,
            lb: prop.lb,
            ub: prop.ub,
            real_rows: prop.real_rows,
            real_rhs: prop.real_rhs,
            c,
            infeasible: true,
            unbounded: false,
            postsolve_log,
        };
    }

    // 消去した変数の上下限を `[0, 0]` に固定する (値は後処理で代入式から復元する)。
    // これをしないと G/h を読む呼び出し側や freevar に、自由度を持つ幽霊列として見える。
    let mut lb = prop.lb;
    let mut ub = prop.ub;
    for step in &postsolve_log {
        if let PostsolveStep::Sub(sub) = step {
            lb[sub.var] = 0.0;
            ub[sub.var] = 0.0;
        }
    }

    // 一般の自由変数消去 (`freevar`)。出現回数に制限なし。
    // 上の固定の後に実行すること (消去済み列を新しい自由変数と誤認しないため)。
    // freevar を実行しない場合の、入力をそのまま返す結果。
    let skipped_freevar = || freevar::FreeVarResult {
        a: a.clone(),
        b: b.clone(),
        c: c.clone(),
        substitutions: Vec::new(),
        fixed: Vec::new(),
        unbounded: false,
        real_rows: prop.real_rows.clone(),
        real_rhs: prop.real_rhs.clone(),
    };
    let free = if env_str!("ENOMOTO_DISABLE_FREEVAR").is_some() {
        skipped_freevar()
    } else {
        let free = timed_step!("freevar", freevar::eliminate_free_variables(n, &a, &b, &c, &lb, &ub, &prop.real_rows, &prop.real_rhs));
        // freevar の「非有界」は改善方向の存在しか示さず、実行可能性は確認していない。
        // 実行不能と非有界を区別する必要がある呼び出し側では判定を採用せず、この段を飛ばす。
        if free.unbounded && !allow_unbounded_verdict { skipped_freevar() } else { free }
    };
    if env_str!("ENOMOTO_DEBUG_FREEVAR").is_some() {
        eprintln!("DEBUG_FREEVAR: eliminated={} fixed={} unbounded={}", free.substitutions.len(), free.fixed.len(), free.unbounded);
    }
    if free.unbounded {
        return extended_unbounded(sc, free.a, free.b, free.c, n);
    }
    a = free.a;
    b = free.b;
    c = free.c;
    for &(j, v) in &free.fixed {
        lb[j] = v;
        ub[j] = v;
    }
    postsolve_log.extend(free.substitutions.into_iter().map(PostsolveStep::Sub));
    // freevar が新たに消去した列も `[0, 0]` に固定する。
    for step in &postsolve_log {
        if let PostsolveStep::Sub(sub) = step {
            lb[sub.var] = 0.0;
            ub[sub.var] = 0.0;
        }
    }

    if env_str!("ENOMOTO_DEBUG_PRESOLVE_HASH").is_some() {
        let (g, h) = propagate::rebuild_g_ref(n, &free.real_rows, &free.real_rhs, &lb, &ub);
        eprintln!("PRESOLVE_HASH {:016x} m_eq={} m_le={} post={}", presolve_output_hash(&a, &b, &g, &h, &c, &lb, &ub), a.nrows(), g.nrows(), postsolve_log.len());
    }

    ExtendedPresolveResult {
        scaling: sc,
        a,
        b,
        lb,
        ub,
        real_rows: free.real_rows,
        real_rhs: free.real_rhs,
        c,
        infeasible: false,
        unbounded: false,
        postsolve_log,
    }
}

/// 縮小後の問題のビットパターンに対する FNV-1a ハッシュ (`ENOMOTO_DEBUG_PRESOLVE_HASH`)。
/// 2 つのビルドで同じ値が出れば、ソルバーへの入力はビット単位で同一。
fn presolve_output_hash(a: &FaerCsr, b: &[f64], g: &FaerCsr, h: &[f64], c: &[f64], lb: &[f64], ub: &[f64]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    // 64 ビット値をリトルエンディアンの 8 バイトとしてハッシュに取り込む。
    let mut eat = |x: u64| {
        for byte in x.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    for m in [a, g] {
        let r = m.as_ref();
        eat(r.nrows() as u64);
        eat(r.ncols() as u64);
        for i in 0..r.nrows() {
            let cols = r.col_indices_of_row_raw(i);
            eat(cols.len() as u64);
            for (&j, &v) in cols.iter().zip(r.values_of_row(i)) {
                eat(j as u64);
                eat(v.to_bits());
            }
        }
    }
    for v in [b, h, c, lb, ub] {
        eat(v.len() as u64);
        for &x in v {
            eat(x.to_bits());
        }
    }
    hash
}
