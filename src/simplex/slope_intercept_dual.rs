//! BFRT 付き傾き・切片二段解法。`solve_lp_dual` の唯一の LP エンジンで、presolve 後の `StdForm`
//! (構造列に真の無限境界が残っていてもいなくても)を解く。`simplex::solve_std_form_decomposed`
//! から独立な連結成分ごとに 1 回呼ばれる。`None` を返した場合(「到達しないはず」の破綻)は
//! `Status::NotSolved` として報告され、代わりに使う別ソルバーは無い。
//!
//! 以下で「論文」と書くのは『双対単体法のみによる線形計画問題の完全な判別：傾き問題と切片問題からなる
//! 2段階法』(`paper.tex`)で、節・式・命題などの番号はその現行版に合わせてある。
//!
//! ## 記号的な `M` (M→∞)
//!
//! 無限の境界 `±inf` を数値の `±M` で切り詰める代わりに、`M` に依存する量をすべてアフィン関数
//! `base + slope * M`([`Affine1`])として持ち、比較は `(slope, base)` の辞書式比較
//! (`Affine1::cmp_lex`)で行う。十分大きい `M` での比較の符号は傾き項で決まり、傾きが等しいとき
//! だけ定数項で決まるので、任意に大きな数値の `M` を使った古典法と同じ判断を、数値的な脆さ無しに
//! 再現できる。`M` の役割は 2 つ: (1) 片側が真の無限でも非基底の構造列を何らかの境界に置けるように
//! する(全スラック初期基底をコストの符号だけで双対実行可能にし、phase 1 を不要にする — [`crash`])、
//! (2) そのような列も BFRT(境界フリップ比率テスト)でフリップできるようにする。
//!
//! どの列を `M` 追跡するかは [`delta_of`] が決める(現在は片側非有界・自由な構造列をすべて
//! フラグする。論文 5.2 節の有界化と同じく、非有界な側はすべて `M` で置き換える)。
//!
//! ## 3 段階アルゴリズム(論文 Algorithm 1)
//!
//! - **段階 A**(傾き問題、論文の式 (7)): `x_B` の `M` 係数 `x^1` だけを、境界の `M` 係数
//!   `l^1, u^1` に対して最適化する([`Phase::A`]、[`ColCache::slope_problem`])。
//!   最適値 `z^1 < 0` なら実行不能か非有界(論文の系 7.3 (i))。
//! - **段階 B**(切片問題、論文の式 (8)): `x^1` を固定し、残った境界 `l^B, u^B` に対する通常の
//!   実数値の双対単体法で定数項 `x^0` を求める([`Phase::B`]、[`ColCache::intercept_problem`])。
//! - **cleanup**([`finish`]): 人工的な `M` 側に残る非基底列を主比率テストで有限側へ移し
//!   (論文の補題 6.6)、[`polish_with_true_bounds`] が真の境界・真のコストで最終確認して解を取り出す。
//!
//! ## 主ループの構成
//!
//! 各反復は chuzr(DSE 重み付きの離基行選択。スコアは [`Score2`] で `M` の次数を考慮)→ BTRAN
//! (`rho = B^-T e_r`)→ 行方向 PRICE(`a_p = rho^T A`)→ chuzc1(比率テストの候補作成)→
//! BFRT(累積フリップ容量が行の逸脱を覆うまで候補をフリップし、Harris 型パス 2 で最終ピボットを
//! 選ぶ)→ FTRAN(`alpha = B^-1 A_q`、DSE の `tau` と融合)→ `x_B(M)`・DSE 重み・被約費用 `d` の
//! 増分更新 → Forrest-Tomlin 更新(再分解トリガ付き)、の順。主実行不能行の集合は
//! [`InfeasibleRows`] で増分維持する(超疎 chuzr)。巡回防止は停滞カウンタによる Bland 規則への
//! 切り替え(`bland_mode`)で行う(論文 6.5 節の費用摂動による辞書式規則ではない)。
//!
//! ## 前提条件(前段で保証される)
//!
//! - 構造列は真に自由(`lb == -inf` かつ `ub == +inf`)でもよい。[`delta_of`]/[`hat_lower`]/
//!   [`hat_upper`] が両側を独立に追跡する。論文の状態 `Z`(3.1 節。`NbStatus::Zero`、値 0 の非基底)は
//!   コスト 0 の自由列の初期配置([`crash`])と cleanup の場合 (A) でだけ使い、`Zero` 列は
//!   chuzc1 で常に適格(比 0)で、BFRT なしに直接ピボットされる(フリップされず、離基で
//!   作られることも無い)。旧 cleanup(`ENOMOTO_LEGACY_CLEANUP=1`)は別の自由基底変数を
//!   追い出す必要があるとき `None` で抜ける。
//! - 片側非有界の構造列は有限側がちょうど 0 になるようシフト済み(`build_std_form_presolved`)
//!   なので、`hat_u_j(M) - hat_l_j(M) = w_j + s_j*M` は `s_j*M` になる(`w_j == 0`)。
//! - スラック列(`<=`/`>=` 行の `[0, inf)`)は `M` フラグしない。真の無限側には決して置かない
//!   ([`hat_upper`] 参照)。

use super::{sparse_lu, InfeasibleRows, NbStatus, SimplexResult, StdForm, Status};
use crate::sparse::sparse_axpy_dense;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::OnceLock;
use crate::params::simplex::{FT_BUMP_LIMIT_FACTOR, FT_CHECK_INTERVAL, FT_MIN_PIVOT, FT_RESIDUAL_TOL, HARRIS_RATIO_TOL, MAX_ITERS_FLOOR, PRIMAL_FEAS_TOL, RESIDUAL_CHECK_MULTIPLIER, STALL_PROGRESS_EPS, STEEPEST_EDGE_FLOOR, TOL};
use crate::params::slope_intercept_dual::{CHUZC1_TOPK, CHUZC1_TOPK_MIN_CANDS, FLIP_TRACK_MIN_M, CHUZC1_FAST_PROBE, SYNTH_CLOCK_MID_REF_MULT, SYNTH_CLOCK_MID_TAU_FRACTION, PREFETCH_DIST, PREFETCH_MIN_COLS, PRICE_DENSE_RESULT_RATIO, SYNTH_CLOCK_DENSE_FRACTION, D_DRIFT_TOL, FT_BUMP_LU_RATIO, PLATEAU_OBJ_REL, D_GROSS_MISMATCH_REL_TOL, FT_MAX_UPDATES_FACTOR, FT_MAX_UPDATES_FLOOR, PARTIAL_TAU_MIN_M, PIVOT_ESCALATION_STEP, SLOPE_TOL, STUCK_ROW_MIN_PIVOT, SYNTH_CLOCK_FACTOR, SYNTH_CLOCK_LARGE_M, SYNTH_CLOCK_LARGE_REF_M, SYNTH_CLOCK_MIN_UPDATES, XB_CHECK_CADENCE, XB_CHECK_CADENCE_LARGE, XB_CHECK_CADENCE_LARGE_DIV, XB_CHECK_CADENCE_LARGE_M, XB_CHECK_INTERVAL, XB_DRIFT_ESCALATION_FACTOR, XB_DRIFT_ESCALATION_STEP, XB_DRIFT_REL_TOL, XB_DRIFT_TOL, XB_DRIFT_TOL_MAX, X_B_SLOPE_NOISE, Z_SLOPE_TOL};
use crate::params::slope_intercept_dual::{
    CHUZR_HEAP_ADAPTIVE_MIN_M, CHUZR_HEAP_ADAPTIVE_RATIO, CHUZR_SHORTLIST_AUTO_DIV, CHUZR_SHORTLIST_AUTO_K, CHUZR_SHORTLIST_AUTO_K_MAX, CHUZR_SHORTLIST_AUTO_MIN_M, CHUZR_SHORTLIST_K, CHUZR_SHORTLIST_MAX_LEN_FACTOR, CHUZR_SHORTLIST_MAX_LEN_SLACK, CHUZR_SHORTLIST_MIN_POOL_FACTOR, COMPACT_ROWS_BLOCK, FTRAN_U_HYPER_DENSITY, FTRAN_U_HYPER_TAU_DENSITY,
    GREATEST_IMPROVEMENT_STALL_DIVISOR, GREATEST_IMPROVEMENT_STALL_MIN, GREATEST_IMPROVEMENT_TOP_K, GROSS_MISMATCH_SCALE_FLOOR, HANDOFF_MAX_ROUNDS, INFEASIBLE_PLATEAU_BUDGET_DIVISOR, INFEASIBLE_PLATEAU_STALL_MULT,
    LEX_REL_TOL, PRICE_COLUMN_DENSITY, PRICE_LIST_DENSITY, SCORE2_STALL_HALFLIFE, STALL_LIMIT_MIN, STALL_LIMIT_PER_ROW, STUCK_ROW_BOOST_FACTOR, STUCK_ROW_BOOST_THRESHOLD, XB_DRIFT_FRESH_FLOOR_FACTOR,
    XB_DRIFT_FRESH_FLOOR_FRAC, XB_DRIFT_MIN_UPDATES, XB_DRIFT_MIN_UPDATES_MULT, XB_DRIFT_REL_K, XB_DRIFT_SAMPLE_GUARD, XB_DRIFT_SAMPLE_K, XB_CHECK_FULL_EVERY, XB_LIST_DENSITY,
};
use crate::params::slope_intercept_dual::{DEGEN_DJ_TOL, DEGEN_SHIFT_RUN, XB_CONSISTENCY_TOL, INFEAS_GUARD_SQRT_W, NOISE_C, NOISE_MIN_SQRT_W, PIVOT_ESCALATE_BELOW_DEFAULT, PIVOT_ESCALATE_FEW_UPDATES, ROLLBACK_MAX, UNCERTIFIED_MAX};

/// `ENOMOTO_PROF_PHASES_EXT` 診断用のフェーズ別計時カウンタ(`simplex::prof_phases` の拡張版)。
/// [`solve_slope_intercept_dual`] の主ループの時間がどこで使われるかを測る。値はすべてナノ秒または
/// 回数の累計で、`reset` で 0 に戻し `report` で標準エラーに出力する。制御フローには使わない。
mod prof_phases {
    use std::sync::atomic::AtomicUsize;
    /// ピボット行 BTRAN(`rho_p`)の累計時間。
    pub(super) static BTRAN: AtomicUsize = AtomicUsize::new(0);
    /// 結果密度の判定だけで密経路に回した FTRAN の数(入力は疎で、入力判定だけなら
    /// Gilbert-Peierls 経路に行ったもの)。0 なら密度判定は一度も効いていない。
    pub(super) static DENSITY_GATE_FTRANS: AtomicUsize = AtomicUsize::new(0);
    /// 入る列の FTRAN の、最後に更新した時点の結果密度の移動平均(`m` に対する千分率)。
    pub(super) static DENSITY_COL_AQ_PPT: AtomicUsize = AtomicUsize::new(0);
    /// BFRT 結合フリップの FTRAN の結果密度の移動平均(千分率)。
    pub(super) static DENSITY_BFRT_PPT: AtomicUsize = AtomicUsize::new(0);
    /// PRICE の累計時間。
    pub(super) static PRICE: AtomicUsize = AtomicUsize::new(0);
    /// chuzr(離基行選択)の累計時間。
    pub(super) static CHUZR: AtomicUsize = AtomicUsize::new(0);
    /// chuzc1(候補作成)の累計時間。
    pub(super) static CHUZC1: AtomicUsize = AtomicUsize::new(0);
    /// BFRT(歩進・パス 2・フリップ)の累計時間。
    pub(super) static BFRT: AtomicUsize = AtomicUsize::new(0);
    /// 入る列の FTRAN の累計時間。
    pub(super) static FTRAN: AtomicUsize = AtomicUsize::new(0);
    /// `x_B(M)` 更新の累計時間。
    pub(super) static XB_UPDATE: AtomicUsize = AtomicUsize::new(0);
    /// DSE 重み更新の累計時間。
    pub(super) static DSE_UPDATE: AtomicUsize = AtomicUsize::new(0);
    /// 双対値 `d` の更新の累計時間。
    pub(super) static DUAL_UPDATE: AtomicUsize = AtomicUsize::new(0);
    /// Forrest-Tomlin 更新の累計時間。
    pub(super) static FT_UPDATE: AtomicUsize = AtomicUsize::new(0);
    /// 再分解(と再同期)の累計時間。
    pub(super) static REFACTOR: AtomicUsize = AtomicUsize::new(0);
    /// 再分解の回数。
    pub(super) static REFACTOR_COUNT: AtomicUsize = AtomicUsize::new(0);
    /// 原因別の再分解回数: updateVerify によるピボット破棄。
    pub(super) static REFACTOR_CAUSE_VERIFY: AtomicUsize = AtomicUsize::new(0);
    /// 原因別: `FtLu::try_update` が確定済みピボットを拒否(対角が `FT_MIN_PIVOT` 未満)。
    pub(super) static REFACTOR_CAUSE_TRY_UPDATE: AtomicUsize = AtomicUsize::new(0);
    /// 原因別: eta ファイルのフィルが上限(`FT_BUMP_LIMIT_FACTOR * m`)超過。
    pub(super) static REFACTOR_CAUSE_BUMP: AtomicUsize = AtomicUsize::new(0);
    /// 原因別: `x_B(M)` の残差ドリフト(`XB_DRIFT_TOL`)。
    pub(super) static REFACTOR_CAUSE_DRIFT: AtomicUsize = AtomicUsize::new(0);
    /// 原因別: `d` のドリフト(`D_DRIFT_TOL`)。
    pub(super) static REFACTOR_CAUSE_D_DRIFT: AtomicUsize = AtomicUsize::new(0);
    /// 原因別: `pivot_grossly_inconsistent`(PRICE と FTRAN のピボット要素の桁違いの不一致)。
    pub(super) static REFACTOR_CAUSE_ILLCOND: AtomicUsize = AtomicUsize::new(0);
    /// 原因別: トリガ (4)、FT 更新回数の上限(`ft_max_updates`)。通常は常に 0 のはず。
    pub(super) static REFACTOR_CAUSE_MAX_UPDATES: AtomicUsize = AtomicUsize::new(0);
    /// 原因別: 更新済みの分解から `Infeasible` を結論しないためのやり直し。
    pub(super) static REFACTOR_CAUSE_INFEAS_CHECK: AtomicUsize = AtomicUsize::new(0);
    /// 原因別: トリガ (5)、合成クロック(`SYNTH_CLOCK_FACTOR`)。
    pub(super) static REFACTOR_CAUSE_CLOCK: AtomicUsize = AtomicUsize::new(0);
    /// 求解中の `lu.update_count()` の最大値(`fetch_max`)。`FT_MAX_UPDATES_FACTOR` の較正用。
    pub(super) static MAX_UPDATE_STREAK: AtomicUsize = AtomicUsize::new(0);
    /// 主ループの反復数。
    pub(super) static ITERS: AtomicUsize = AtomicUsize::new(0);
    /// 実ピボット前にフリップした候補数(`best_idx`)の累計(平均 BFRT バッチサイズ用)。
    pub(super) static BFRT_FLIPS: AtomicUsize = AtomicUsize::new(0);
    /// 各反復の chuzr 開始時の実行不能行プールの大きさの累計。
    pub(super) static INFEASIBLE_POOL: AtomicUsize = AtomicUsize::new(0);
    /// 入る列 `q` がピボット前に自分の `M` 側にあった回数(M フラグ列が `M` 側を離れる唯一の経路)。
    pub(super) static M_EXIT: AtomicUsize = AtomicUsize::new(0);
    /// Harris パス 2 の窓に、選ばれなかった `M` 側の候補があった回数。
    pub(super) static HARRIS_WINDOW_M_MISS: AtomicUsize = AtomicUsize::new(0);
    /// BFRT フリップで列が `M` 側へ移った回数。
    pub(super) static M_ENTER_VIA_FLIP: AtomicUsize = AtomicUsize::new(0);
    /// 入る列の被約費用がほぼ 0 だった(退化)ピボットの数。
    pub(super) static DEGENERATE_PIVOTS: AtomicUsize = AtomicUsize::new(0);
    /// Harris パス 2 の窓の大きさの累計(`ITERS` で割って平均)。
    pub(super) static HARRIS_WINDOW_SIZE_SUM: AtomicUsize = AtomicUsize::new(0);
    /// 実行不能行プールが前反復より大きくなった回数。
    pub(super) static POOL_GREW: AtomicUsize = AtomicUsize::new(0);
    /// 実行不能行プールが前反復より小さくなった回数。
    pub(super) static POOL_SHRANK: AtomicUsize = AtomicUsize::new(0);
    /// 実行不能行プールの大きさが前反復と同じだった回数。
    pub(super) static POOL_SAME: AtomicUsize = AtomicUsize::new(0);
    /// 選ばれた行について、維持している DSE 重みと真の値 `‖rho‖^2` を比較した回数。
    pub(super) static DSE_CHECKED: AtomicUsize = AtomicUsize::new(0);
    /// DSE 重みの相対誤差が 1% 未満だった回数。
    pub(super) static DSE_ERR_LT_1PCT: AtomicUsize = AtomicUsize::new(0);
    /// 相対誤差が 1% 以上 10% 未満だった回数。
    pub(super) static DSE_ERR_LT_10PCT: AtomicUsize = AtomicUsize::new(0);
    /// 相対誤差が 10% 以上 100% 未満だった回数。
    pub(super) static DSE_ERR_LT_100PCT: AtomicUsize = AtomicUsize::new(0);
    /// 相対誤差が 100% 以上だった回数。
    pub(super) static DSE_ERR_GE_100PCT: AtomicUsize = AtomicUsize::new(0);
    /// (`ENOMOTO_PROF_PHASES_EXT_WORK=1` のみ)`rho_p` の非ゼロ数の累計。
    pub(super) static STAT_RHO_NNZ: AtomicUsize = AtomicUsize::new(0);
    /// (WORK)PRICE の内側ループで訪れた要素数の累計。
    pub(super) static STAT_PRICE_ENTRIES: AtomicUsize = AtomicUsize::new(0);
    /// (WORK)PRICE が触れた列数の累計。
    pub(super) static STAT_TOUCHED: AtomicUsize = AtomicUsize::new(0);
    /// (WORK)PRICE が触れた列のうち非基底列の数の累計。
    pub(super) static STAT_TOUCHED_NONBASIC: AtomicUsize = AtomicUsize::new(0);
    /// (WORK)chuzc1 の候補数の累計。
    pub(super) static STAT_CANDS: AtomicUsize = AtomicUsize::new(0);
    /// (WORK)`alpha_q` の非ゼロ数の累計。
    pub(super) static STAT_ALPHA_NNZ: AtomicUsize = AtomicUsize::new(0);
    /// (WORK)DSE の `tau` の非ゼロ数の累計。
    pub(super) static STAT_TAU_NNZ: AtomicUsize = AtomicUsize::new(0);
    /// 融合しない場合の DSE `tau` FTRAN の累計時間(`DSE_UPDATE` の一部)。
    pub(super) static DSE_FTRAN: AtomicUsize = AtomicUsize::new(0);
    /// chuzc1 のヒープ構築の累計時間。
    pub(super) static CHUZC1_HEAP: AtomicUsize = AtomicUsize::new(0);
    /// 行数 `m`(作業量の出力用)。
    pub(super) static STAT_M: AtomicUsize = AtomicUsize::new(0);
    /// (WORK)PRICE で訪れた要素のうち非基底列のものの累計。
    pub(super) static STAT_PRICE_ENTRIES_NB: AtomicUsize = AtomicUsize::new(0);
    /// 全カウンタを 0 に戻す(求解の開始時に呼ぶ)。
    pub(super) fn reset() {
        use std::sync::atomic::Ordering::Relaxed;
        for c in [
            &BTRAN,
            &PRICE,
            &CHUZR,
            &CHUZC1,
            &BFRT,
            &FTRAN,
            &XB_UPDATE,
            &DSE_UPDATE,
            &DUAL_UPDATE,
            &FT_UPDATE,
            &REFACTOR,
            &REFACTOR_COUNT,
            &REFACTOR_CAUSE_VERIFY,
            &REFACTOR_CAUSE_TRY_UPDATE,
            &REFACTOR_CAUSE_BUMP,
            &REFACTOR_CAUSE_DRIFT,
            &REFACTOR_CAUSE_D_DRIFT,
            &REFACTOR_CAUSE_ILLCOND,
            &REFACTOR_CAUSE_MAX_UPDATES,
            &REFACTOR_CAUSE_INFEAS_CHECK,
            &REFACTOR_CAUSE_CLOCK,
            &MAX_UPDATE_STREAK,
            &ITERS,
            &BFRT_FLIPS,
            &INFEASIBLE_POOL,
            &M_EXIT,
            &HARRIS_WINDOW_M_MISS,
            &M_ENTER_VIA_FLIP,
            &DEGENERATE_PIVOTS,
            &HARRIS_WINDOW_SIZE_SUM,
            &POOL_GREW,
            &POOL_SHRANK,
            &POOL_SAME,
            &DSE_CHECKED,
            &DSE_ERR_LT_1PCT,
            &DSE_ERR_LT_10PCT,
            &DSE_ERR_LT_100PCT,
            &DSE_ERR_GE_100PCT,
            &STAT_RHO_NNZ,
            &STAT_PRICE_ENTRIES,
            &STAT_TOUCHED,
            &STAT_TOUCHED_NONBASIC,
            &STAT_CANDS,
            &STAT_ALPHA_NNZ,
            &STAT_TAU_NNZ,
            &DSE_FTRAN,
            &CHUZC1_HEAP,
            &STAT_M,
            &STAT_PRICE_ENTRIES_NB,
            &DENSITY_GATE_FTRANS,
            &DENSITY_COL_AQ_PPT,
            &DENSITY_BFRT_PPT,
        ] {
            c.store(0, Relaxed);
        }
    }

    /// `simplex::solve_lp_dual` の `PROF_PHASES` と同じ形式の報告を標準エラーに出力する
    /// (行単位で比較できる)。`wall_ns` は求解全体の経過時間(ナノ秒)。
    pub(super) fn report(wall_ns: usize) {
        use std::sync::atomic::Ordering::Relaxed;
        let iters = ITERS.load(Relaxed).max(1);
        let phases: [(&str, usize); 10] = [
            ("btran(rho_p)", BTRAN.load(Relaxed)),
            ("price", PRICE.load(Relaxed)),
            ("chuzr", CHUZR.load(Relaxed)),
            ("chuzc1", CHUZC1.load(Relaxed)),
            ("bfrt", BFRT.load(Relaxed)),
            ("ftran", FTRAN.load(Relaxed)),
            ("xb_update", XB_UPDATE.load(Relaxed)),
            ("dse_update", DSE_UPDATE.load(Relaxed)),
            ("dual_update", DUAL_UPDATE.load(Relaxed)),
            ("ft_update", FT_UPDATE.load(Relaxed)),
        ];
        let accounted: usize = phases.iter().map(|&(_, ns)| ns).sum::<usize>() + REFACTOR.load(Relaxed);
        eprintln!(
            "PROF_PHASES_EXT wall={:.3}ms iters={iters} ({:.1}us/iter) accounted={:.1}% of wall refactor_count={} (verify={} try_update={} bump={} drift={} d_drift={} illcond={} max_updates={} infeas_check={} clock={}) max_update_streak={}",
            wall_ns as f64 / 1e6,
            wall_ns as f64 / 1e3 / iters as f64,
            100.0 * accounted as f64 / wall_ns.max(1) as f64,
            REFACTOR_COUNT.load(Relaxed),
            REFACTOR_CAUSE_VERIFY.load(Relaxed),
            REFACTOR_CAUSE_TRY_UPDATE.load(Relaxed),
            REFACTOR_CAUSE_BUMP.load(Relaxed),
            REFACTOR_CAUSE_DRIFT.load(Relaxed),
            REFACTOR_CAUSE_D_DRIFT.load(Relaxed),
            REFACTOR_CAUSE_ILLCOND.load(Relaxed),
            REFACTOR_CAUSE_MAX_UPDATES.load(Relaxed),
            REFACTOR_CAUSE_INFEAS_CHECK.load(Relaxed),
            REFACTOR_CAUSE_CLOCK.load(Relaxed),
            MAX_UPDATE_STREAK.load(Relaxed)
        );
        eprintln!(
            "  dse_ftran(unfused)={:.1}us/iter chuzc1_heap={:.1}us/iter",
            DSE_FTRAN.load(Relaxed) as f64 / 1e3 / iters as f64,
            CHUZC1_HEAP.load(Relaxed) as f64 / 1e3 / iters as f64
        );
        if STAT_M.load(Relaxed) != 0 && STAT_RHO_NNZ.load(Relaxed) != 0 {
            eprintln!(
                "  work/iter (m={}): rho_nnz={:.1} price_entries={:.1} (nonbasic {:.1}) touched={:.1} touched_nonbasic={:.1} cands={:.1} alpha_nnz={:.1} tau_nnz={:.1}",
                STAT_M.load(Relaxed),
                STAT_RHO_NNZ.load(Relaxed) as f64 / iters as f64,
                STAT_PRICE_ENTRIES.load(Relaxed) as f64 / iters as f64,
                STAT_PRICE_ENTRIES_NB.load(Relaxed) as f64 / iters as f64,
                STAT_TOUCHED.load(Relaxed) as f64 / iters as f64,
                STAT_TOUCHED_NONBASIC.load(Relaxed) as f64 / iters as f64,
                STAT_CANDS.load(Relaxed) as f64 / iters as f64,
                STAT_ALPHA_NNZ.load(Relaxed) as f64 / iters as f64,
                STAT_TAU_NNZ.load(Relaxed) as f64 / iters as f64
            );
        }
        eprintln!(
            "  avg_bfrt_flips/iter={:.3} avg_infeasible_pool/iter={:.1}",
            BFRT_FLIPS.load(Relaxed) as f64 / iters as f64,
            INFEASIBLE_POOL.load(Relaxed) as f64 / iters as f64
        );
        {
            // ピボット順の再利用(`sparse_lu::factorize_reusing`): 試行回数・採用回数・試行時間・
            // 記録されたピボット行が数値的に空で選び直した段の数など。
            use crate::simplex::sparse_lu;
            let attempts = sparse_lu::PROF_REBUILD_ATTEMPTS.load(Relaxed);
            let accepted = sparse_lu::PROF_REBUILD_ACCEPTED.load(Relaxed);
            eprintln!(
                "  pivot_order_reuse: attempts={attempts} accepted={accepted} ({:.1}%) attempt_time={:.3}ms row_repicks={} fail(singular={} fill={}) backoff_skips={} nnz_reuse_avg={:.0} nnz_full_avg={:.0}",
                100.0 * accepted as f64 / attempts.max(1) as f64,
                sparse_lu::PROF_REBUILD_NS.load(Relaxed) as f64 / 1e6,
                sparse_lu::PROF_REBUILD_ROW_REPICKS.load(Relaxed),
                sparse_lu::PROF_REBUILD_FAIL_SINGULAR.load(Relaxed),
                sparse_lu::PROF_REBUILD_FAIL_FILL.load(Relaxed),
                sparse_lu::PROF_REBUILD_BACKOFF_SKIPS.load(Relaxed),
                sparse_lu::PROF_REBUILD_ACCEPTED_NNZ.load(Relaxed) as f64 / accepted.max(1) as f64,
                sparse_lu::PROF_FULL_NNZ.load(Relaxed) as f64
                    / sparse_lu::PROF_FULL_COUNT.load(Relaxed).max(1) as f64,
            );
            eprintln!(
                "  lu_dense_switch: count={} rows={}",
                sparse_lu::PROF_DENSE_SWITCH.load(Relaxed),
                sparse_lu::PROF_DENSE_SWITCH_ROWS.load(Relaxed),
            );
        }
        eprintln!(
            "  density_gate_ftrans={} final_expected_density col_aq={:.3} bfrt={:.3}",
            DENSITY_GATE_FTRANS.load(Relaxed),
            DENSITY_COL_AQ_PPT.load(Relaxed) as f64 / 1000.0,
            DENSITY_BFRT_PPT.load(Relaxed) as f64 / 1000.0
        );
        {
            use super::super::sparse_lu as lu;
            eprintln!(
                "  btran_l scatter={} gather={}",
                lu::PROF_BTRAN_L_SCATTER.load(Relaxed),
                lu::PROF_BTRAN_L_GATHER.load(Relaxed),
            );
            // Markowitz ピボット探索のカウンタ(`ENOMOTO_PROF_TRIANGULAR` で `lu.rs` の走査タイマーが有効)。
            let steps = lu::PROF_TOTAL_STEPS.load(Relaxed);
            eprintln!(
                "  pivot_search steps={steps} search_limit_hits={} ({:.1}%) candidates={} (avg {:.2}/step) scan={:.3}ms",
                lu::PROF_SEARCH_LIMIT_STEPS.load(Relaxed),
                100.0 * lu::PROF_SEARCH_LIMIT_STEPS.load(Relaxed) as f64 / steps.max(1) as f64,
                lu::PROF_SEARCH_CANDIDATES.load(Relaxed),
                lu::PROF_SEARCH_CANDIDATES.load(Relaxed) as f64 / steps.max(1) as f64,
                lu::PROF_BUCKET_SCAN_NS.load(Relaxed) as f64 / 1e6,
            );
            // `docs/lu_comparison_enomoto_vs_highs.md` §2.4: 増分 `colFixMax` で省けたはずの作業量と、この求解のピボット閾値が初期値から
            // 変わったか(段階的引き上げの有無)。
            eprintln!(
                "  col_max_abs rescan_entries={} pivot_threshold={} (escalations={})",
                lu::PROF_COLMAX_RESCAN_ENTRIES.load(Relaxed),
                lu::pivot_threshold(),
                lu::PROF_PIVOT_ESCALATIONS.load(Relaxed),
            );
        }
        eprintln!(
            "  m_exit(q_was_m)={} harris_window_m_miss={} m_enter_via_flip={}",
            M_EXIT.load(Relaxed),
            HARRIS_WINDOW_M_MISS.load(Relaxed),
            M_ENTER_VIA_FLIP.load(Relaxed)
        );
        eprintln!(
            "  degenerate_pivots={} ({:.1}% of iters) avg_harris_window_size={:.3}",
            DEGENERATE_PIVOTS.load(Relaxed),
            100.0 * DEGENERATE_PIVOTS.load(Relaxed) as f64 / iters as f64,
            HARRIS_WINDOW_SIZE_SUM.load(Relaxed) as f64 / iters as f64
        );
        eprintln!(
            "  pool_grew={} ({:.1}%) pool_shrank={} ({:.1}%) pool_same={} ({:.1}%)",
            POOL_GREW.load(Relaxed),
            100.0 * POOL_GREW.load(Relaxed) as f64 / iters as f64,
            POOL_SHRANK.load(Relaxed),
            100.0 * POOL_SHRANK.load(Relaxed) as f64 / iters as f64,
            POOL_SAME.load(Relaxed),
            100.0 * POOL_SAME.load(Relaxed) as f64 / iters as f64
        );
        let dse_checked = DSE_CHECKED.load(Relaxed).max(1);
        eprintln!(
            "  dse_rel_err: checked={} <1%={:.1}% <10%={:.1}% <100%={:.1}% >=100%={:.1}%",
            DSE_CHECKED.load(Relaxed),
            100.0 * DSE_ERR_LT_1PCT.load(Relaxed) as f64 / dse_checked as f64,
            100.0 * DSE_ERR_LT_10PCT.load(Relaxed) as f64 / dse_checked as f64,
            100.0 * DSE_ERR_LT_100PCT.load(Relaxed) as f64 / dse_checked as f64,
            100.0 * DSE_ERR_GE_100PCT.load(Relaxed) as f64 / dse_checked as f64
        );
        for (name, ns) in phases {
            eprintln!(
                "  {name:20} {:8.3}ms  {:5.1}% of wall  {:.3}us/iter",
                ns as f64 / 1e6,
                100.0 * ns as f64 / wall_ns.max(1) as f64,
                ns as f64 / 1e3 / iters as f64
            );
        }
        eprintln!(
            "  {:20} {:8.3}ms  {:5.1}% of wall  {:.3}us/iter",
            "refactor",
            REFACTOR.load(Relaxed) as f64 / 1e6,
            100.0 * REFACTOR.load(Relaxed) as f64 / wall_ns.max(1) as f64,
            REFACTOR.load(Relaxed) as f64 / 1e3 / iters as f64
        );
    }
}

/// `$enabled` が真なら `$body` の実行時間(ナノ秒)を計測して `$counter` に加算し、`$body` の値を
/// 返す(`simplex.rs` の `timed!` と同じもの。モジュール境界をまたいで共有していない)。
/// `$enabled` は反復ごとに一度読んだ `bool`。
macro_rules! timed {
    ($enabled:expr, $counter:expr, $body:expr) => {{
        if $enabled {
            let __t0 = std::time::Instant::now();
            let __r = $body;
            $counter.fetch_add(__t0.elapsed().as_nanos() as usize, std::sync::atomic::Ordering::Relaxed);
            __r
        } else {
            $body
        }
    }};
}

/// トリガ (4) の閾値: FT 更新回数の上限 `max(FT_MAX_UPDATES_FACTOR * m, FT_MAX_UPDATES_FLOOR)`
/// (`m` に比例させる。[`FT_MAX_UPDATES_FACTOR`] 参照)。
#[inline]
fn ft_max_updates(m: usize) -> usize {
    ((tunable!("ENOMOTO_T_FT_MAX_UPDATES_FACTOR", FT_MAX_UPDATES_FACTOR, f64) * m as f64) as usize).max(FT_MAX_UPDATES_FLOOR)
}

/// [`SYNTH_CLOCK_FACTOR`](`ENOMOTO_SYNTH_CLOCK_FACTOR` で上書き可、一度だけ解析してキャッシュ)。
/// 再較正のための調整つまみで、出荷時の挙動は [`SYNTH_CLOCK_FACTOR`] の値。
fn synth_clock_factor() -> f64 {
    static FACTOR: OnceLock<f64> = OnceLock::new();
    *FACTOR.get_or_init(|| {
        env_str!("ENOMOTO_SYNTH_CLOCK_FACTOR")
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|f| f.is_finite() && *f > 0.0)
            .unwrap_or(SYNTH_CLOCK_FACTOR)
    })
}

/// トリガ (5): 合成クロックによる再分解判定。FT 更新回数が [`SYNTH_CLOCK_MIN_UPDATES`] 以上で、
/// 前回の分解以降に蓄積した求解側の tick が `synth_clock_factor() * build_tick` 以上なら真
/// (現在の分解に対する FTRAN/BTRAN の手間が再分解の手間に達したとみなす)。主ループと
/// [`polish_with_true_bounds`] で共有する(毎ピボット呼べるほど安価)。
#[inline]
fn synth_clock_should_refactor(lu: &sparse_lu::FtLu) -> bool {
    synth_clock_should_refactor_density(lu, SynthDensity::Sparse)
}

/// 合成クロックの係数を選ぶための求解結果の密度の区分 (square41 / ex10 報告の策11 と pds-100 報告の策5(b))。
#[derive(Clone, Copy, PartialEq, Eq)]
enum SynthDensity {
    /// 求解結果が超疎 (stormG2_1000: DSE `tau` の非ゼロ率 1e-5 程度)。策10 の `sqrt(m / REF)` 倍をそのまま掛ける。
    Sparse,
    /// 中程度 (pds-100: `tau` の非ゼロ率 1〜5%)。基準行数を [`SYNTH_CLOCK_MID_REF_MULT`] 倍にして倍率を小さくする。
    Mid,
    /// 密 (ex10: 入る列の結果の 80% が非ゼロ)。倍率を掛けない。
    Dense,
}

/// [`synth_clock_should_refactor`] に求解結果の密度の区分を加えたもの。策10 の `sqrt(m)` 倍は「求解の `O(m)`
/// パスを消したので実際の求解の手間は `m` によらない」ことが前提で、結果が密になるほど前提が崩れる:
/// 密なら掛けず (策11)、中程度なら基準行数を大きくして倍率を下げる (pds-100 は FT 更新が積もるにつれて
/// `R` 段と `tau` の密な反復が重くなり、2,270 反復ごとの再分解では遅すぎた)。
#[inline]
fn synth_clock_should_refactor_density(lu: &sparse_lu::FtLu, density: SynthDensity) -> bool {
    let factor = match density {
        SynthDensity::Sparse => synth_clock_factor_for(lu.dim()),
        SynthDensity::Mid => synth_clock_factor_for_ref(lu.dim(), tunable!("ENOMOTO_T_SYNTH_CLOCK_MID_REF_MULT", SYNTH_CLOCK_MID_REF_MULT, f64)),
        SynthDensity::Dense => synth_clock_factor(),
    };
    lu.update_count() >= tunable!("ENOMOTO_T_SYNTH_CLOCK_MIN_UPDATES", SYNTH_CLOCK_MIN_UPDATES, usize) && (lu.synth_tick() as f64) >= factor * (lu.build_tick().max(1) as f64)
}

/// 行数 `m` の問題に使う合成クロックの係数: [`synth_clock_factor`]、ただし策10 で
/// `m >= SYNTH_CLOCK_LARGE_M` なら `sqrt(m / SYNTH_CLOCK_LARGE_REF_M)` 倍する。
#[inline]
fn synth_clock_factor_for(m: usize) -> f64 {
    synth_clock_factor_for_ref(m, 1.0)
}

/// [`synth_clock_factor_for`] の基準行数 `SYNTH_CLOCK_LARGE_REF_M` を `ref_mult` 倍したもの。
#[inline]
fn synth_clock_factor_for_ref(m: usize, ref_mult: f64) -> f64 {
    let large_m = tunable!("ENOMOTO_T_SYNTH_CLOCK_LARGE_M", SYNTH_CLOCK_LARGE_M, usize);
    if large_m > 0 && m >= large_m {
        let ref_m = tunable!("ENOMOTO_T_SYNTH_CLOCK_LARGE_REF_M", SYNTH_CLOCK_LARGE_REF_M, usize).max(1) as f64 * ref_mult;
        synth_clock_factor() * (m as f64 / ref_m).sqrt().max(1.0)
    } else {
        synth_clock_factor()
    }
}

/// テスト専用の計測: cleanup 補題で実際に行った基底交換(`finish` の「塞ぐ行あり」分岐)の
/// 回数を数え、単体テストがその経路を通ったことを確かめられるようにする。
///
/// テストは並列実行されるので(プロセス全体のカウンタだと他のテストの値が混ざる)、
/// スレッドごとのカウンタを `AtomicUsize` と同じ `load`/`fetch_add` の形で提供する。
#[cfg(test)]
pub(crate) struct ThreadLocalCounter;
#[cfg(test)]
thread_local! {
    /// このスレッドでの cleanup ピボット数。
    static CLEANUP_PIVOTS_TL: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
impl ThreadLocalCounter {
    /// 現在の値を返す(`Ordering` は無視)。
    pub(crate) fn load(&self, _: std::sync::atomic::Ordering) -> usize {
        CLEANUP_PIVOTS_TL.with(|c| c.get())
    }
    /// `v` を加算し、加算前の値を返す(`Ordering` は無視)。
    pub(crate) fn fetch_add(&self, v: usize, _: std::sync::atomic::Ordering) -> usize {
        CLEANUP_PIVOTS_TL.with(|c| {
            let old = c.get();
            c.set(old + v);
            old
        })
    }
}
/// テスト専用: cleanup 補題の基底交換回数(スレッドごと)。
#[cfg(test)]
pub(crate) static CLEANUP_PIVOTS: ThreadLocalCounter = ThreadLocalCounter;

/// テスト専用の計測: BFRT の歩進が実ピボットの前にフリップした候補数の累計(全求解・全反復)。
/// 結合フリップによる `x_B(M)` の増分更新が実際に仕事をしたことを単体テストで確かめるために使う。
#[cfg(test)]
pub(crate) static COMBINED_FLIP_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// `x_B(M)` の傾き成分 `v` が [`X_B_SLOPE_NOISE`] 未満なら 0 に丸める(LU/更新の雑音を消す)。
#[inline]
fn snap_slope(v: f64) -> f64 {
    if v.abs() < tunable!("ENOMOTO_T_X_B_SLOPE_NOISE", X_B_SLOPE_NOISE, f64) {
        0.0
    } else {
        v
    }
}

/// スライスの全要素に [`snap_slope`] を適用する。
#[inline]
fn snap_slopes(v: &mut [f64]) {
    for x in v.iter_mut() {
        *x = snap_slope(*x);
    }
}

/// `base + slope * M`(`M -> +∞` の概念上の値で、数値を代入することは無い)。比較はすべて
/// `(slope, base)` の辞書式で行う(モジュール先頭の説明参照)。
#[derive(Clone, Copy, Debug)]
struct Affine1 {
    /// `M` に依存しない定数項。
    base: f64,
    /// `M` の係数。
    slope: f64,
}

impl Affine1 {
    /// 0(定数項・係数とも 0)。
    const ZERO: Affine1 = Affine1 { base: 0.0, slope: 0.0 };

    /// `base + slope * M` を作る。
    #[inline]
    fn new(base: f64, slope: f64) -> Self {
        Affine1 { base, slope }
    }

    /// 論文の `≻`(6.1 節): `(slope, base)` を辞書式に比較する。十分大きい任意の `M` について
    /// `self.value(M) > other.value(M)` のときちょうど `Greater` を返す。
    ///
    /// 両成分とも完全一致ではなく相対許容誤差 [`LEX_REL_TOL`] で比較する(尺度は
    /// `max(|a|, |b|, 1)`): 数学的に等しい 2 つの値(例: 行の逸脱と BFRT の累積フリップ容量)は
    /// LU 求解と容量の和という別経路で計算され、数 ULP ずれるため。
    #[inline]
    fn cmp_lex(&self, other: &Affine1) -> std::cmp::Ordering {
        let slope_scale = self.slope.abs().max(other.slope.abs()).max(1.0);
        let slope_diff = self.slope - other.slope;
        if slope_diff.abs() > LEX_REL_TOL * slope_scale {
            return if slope_diff > 0.0 { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less };
        }
        let base_scale = self.base.abs().max(other.base.abs()).max(1.0);
        let base_diff = self.base - other.base;
        if base_diff.abs() > LEX_REL_TOL * base_scale {
            if base_diff > 0.0 { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less }
        } else {
            std::cmp::Ordering::Equal
        }
    }

    /// `self.cmp_lex(&Affine1::ZERO) == Ordering::Greater` と正確に同じ判定を、`other == 0` の場合に
    /// 許容誤差の計算を畳み込んで行う(主実行可能性の再判定ごとに 2 回呼ばれるホットパス)。
    /// 0 との比較では傾きの判定は `LEX_REL_TOL < |s| < inf` に帰着し(`|s| = inf` と NaN は偽)、
    /// 定数項も同様で、`Greater` には決め手となった成分が正であることも必要。
    #[inline]
    fn gt_zero(&self) -> bool {
        let sa = self.slope.abs();
        if sa > LEX_REL_TOL && sa < f64::INFINITY {
            return self.slope > 0.0;
        }
        self.base > LEX_REL_TOL && self.base < f64::INFINITY
    }

    /// 成分ごとの和。
    #[inline]
    fn add(self, other: Affine1) -> Affine1 {
        Affine1::new(self.base + other.base, self.slope + other.slope)
    }

    /// 成分ごとの差。
    #[inline]
    fn sub(self, other: Affine1) -> Affine1 {
        Affine1::new(self.base - other.base, self.slope - other.slope)
    }

    /// スカラー倍(`k` 倍)。
    #[inline]
    fn scale(self, k: f64) -> Affine1 {
        Affine1::new(self.base * k, self.slope * k)
    }
}

/// BFRT の累積フリップ容量 `cum` が行の逸脱 `w_r` に追いついた(覆った)か、つまり歩進をここで
/// 止めるべきかを返す。[`Affine1::cmp_lex`] と同じ辞書式比較だが、定数項の許容誤差を
/// この行の基底値の大きさ `x_b_base_r` で尺度付けする(`PRIMAL_FEAS_TOL * max(|w_r.base|,
/// |x_b_base_r|, 1)`)。FT のドリフト検査は再分解間に `x_B` の誤差を `XB_DRIFT_TOL_MAX` まで
/// 許すので、値の大きい行では固定の絶対許容誤差だと通常の丸めが解消不能な不足に見えるため。
#[inline]
fn bfrt_reached(w_r: Affine1, cum: Affine1, x_b_base_r: f64) -> bool {
    let slope_scale = w_r.slope.abs().max(cum.slope.abs()).max(1.0);
    let slope_diff = w_r.slope - cum.slope;
    if slope_diff.abs() > LEX_REL_TOL * slope_scale {
        return slope_diff <= 0.0;
    }
    let base_diff = w_r.base - cum.base;
    base_diff <= tunable!("ENOMOTO_T_PRIMAL_FEAS_TOL", PRIMAL_FEAS_TOL, f64) * w_r.base.abs().max(x_b_base_r.abs()).max(1.0)
}

/// steepest-edge/Devex のスコア `Δ_i(M)^2 / w_i`(論文 5.4 節 Step 2(b) の行の重み付き選択を 6.1 節 (b') の辞書式比較に拡張したもの)の比較用表現。候補行では
/// `Δ_i ≻ 0` で、正の範囲では 2 乗は単調増加なので `Δ_i/sqrt(w_i)` で順位付けしてよく、
/// これは `M` のアフィン関数(`w_i` は `M` に依存しない)。よってスコアは
/// `(slope/sqrt(w), base/sqrt(w))` の組を [`Affine1`] と同じ辞書式順で比べる。
///
/// `M` を含まない行(`slope == 0`: delta = 0 以降のほぼ全行)は平方根を取らずに古典的な
/// `base^2/w` をキーとして持つ(`squared`)。組の形は `M` 倍の行が比較に関わるときだけ使う。
#[derive(Clone, Copy, Debug)]
struct Score2 {
    /// `slope/sqrt(w)`(`squared` のときは正確に `0.0`)。
    slope: f64,
    /// `squared` なら `base^2/w`、そうでなければ `base/sqrt(w)`。
    key: f64,
    /// `key` が `base^2/w`(傾き 0 の行)か。
    squared: bool,
}

impl Score2 {
    /// 逸脱 `dev` と DSE 重み `w`(`STEEPEST_EDGE_FLOOR` で下限を取る)からスコアを作る。
    #[inline]
    fn new(dev: Affine1, w: f64) -> Self {
        let w = w.max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
        if dev.slope == 0.0 {
            return Score2 { slope: 0.0, key: dev.base * dev.base / w, squared: true };
        }
        let s = 1.0 / w.sqrt();
        Score2 { slope: dev.slope * s, key: dev.base * s, squared: false }
    }

    /// `base/sqrt(w)`(傾き 0 の逸脱では `base > 0`)。
    #[inline]
    fn linear_key(&self) -> f64 {
        if self.squared {
            self.key.sqrt()
        } else {
            self.key
        }
    }

    /// [`Affine1::cmp_lex`] と同じ順序と相対許容誤差で比較する(数学的に同点のスコアは無関係な
    /// FTRAN/BTRAN の連鎖から来るので数 ULP 以上は一致しない)。`c2_tol` はこの比較だけ先頭
    /// (傾き)項の相対許容誤差を上書きし、第 2 項は常に [`LEX_REL_TOL`] を使う。
    /// 実験的(`ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL`): 主ループは未解決の M フラグ列数から `c2_tol` を
    /// 反復ごとに決める。既定(`1e-9`)では通常の固定許容誤差の比較。
    #[inline]
    fn cmp_lex(&self, other: &Score2, c2_tol: f64) -> std::cmp::Ordering {
        /// `a` と `b` を尺度 `max(|a|, |b|, 1)` の相対許容誤差 `tol` で比較する。
        #[inline(always)]
        fn cmp_tol(a: f64, b: f64, tol: f64) -> std::cmp::Ordering {
            let scale = a.abs().max(b.abs()).max(1.0);
            let diff = a - b;
            if diff.abs() > tol * scale {
                if diff > 0.0 { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less }
            } else {
                std::cmp::Ordering::Equal
            }
        }
        if self.squared && other.squared {
            return cmp_tol(self.key, other.key, LEX_REL_TOL);
        }
        match cmp_tol(self.slope, other.slope, c2_tol) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        if self.squared == other.squared {
            cmp_tol(self.key, other.key, LEX_REL_TOL)
        } else {
            cmp_tol(self.linear_key(), other.linear_key(), LEX_REL_TOL)
        }
    }
}

/// 構造列 `j` のどちら側を `M` で追跡するか(論文の補題 6.1 を、自由列では両側追跡に一般化)。
/// スラック列では常に `None`(呼び出し側が強制する)。現在は S 制限なし: 片側非有界の列は
/// コストの符号にかかわらずすべてフラグする。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MSide {
    /// `M` 追跡しない(両境界とも有限、またはスラック列)。
    None,
    /// `lb[j] == -inf` のみ: `Lower` に置くと `x_j = -M`。
    Lower,
    /// `ub[j] == +inf` のみ: `Upper` に置くと `x_j = +M`。
    Upper,
    /// 自由列(`lb[j] == -inf` かつ `ub[j] == +inf`): 両側を追跡する。
    Both,
}

impl MSide {
    /// 下側(`Lower`)を `M` 追跡するか。
    #[inline]
    fn has_lower(self) -> bool {
        matches!(self, MSide::Lower | MSide::Both)
    }

    /// 上側(`Upper`)を `M` 追跡するか。
    #[inline]
    fn has_upper(self) -> bool {
        matches!(self, MSide::Upper | MSide::Both)
    }

    /// どちらかの側を `M` 追跡するか。
    #[inline]
    fn is_flagged(self) -> bool {
        !matches!(self, MSide::None)
    }
}

/// 構造列 `j` の `M` 追跡側([`MSide`])を境界の無限性から決める。スラック列には呼ばない。
fn delta_of(std: &StdForm, j: usize) -> MSide {
    match (std.lb[j] == f64::NEG_INFINITY, std.ub[j] == f64::INFINITY) {
        (true, true) => MSide::Both,
        (true, false) => MSide::Lower,
        (false, true) => MSide::Upper,
        (false, false) => MSide::None,
    }
}

/// 非基底列 `j` を `Lower` に置いたときの値。構造列(`j < n_orig`)で `lb[j] == -inf` なら
/// `-M`(`Affine1::new(0.0, -1.0)`、`ub[j]` も無限の自由列でも同様)、そうでなければ有限の
/// 境界値。`None` はスラック(`j >= n_orig`)の `lb` が無限の場合だけ(実際には起きない)。
fn hat_lower(std: &StdForm, n_orig: usize, j: usize) -> Option<Affine1> {
    if std.lb[j] == f64::NEG_INFINITY {
        if j < n_orig {
            Some(Affine1::new(0.0, -1.0))
        } else {
            None
        }
    } else {
        Some(Affine1::new(std.lb[j], 0.0))
    }
}

/// 非基底列 `j` を `Upper` に置いたときの値([`hat_lower`] と対称)。構造列で `ub[j] == +inf` なら
/// `+M`。`None` は真の(`M` でない)無限、つまり `<=`/`>=` 行のスラックの上側だけで、その側には
/// 非基底列を置かず、フリップもしない(古典的な有界双対単体法と同じ)。
fn hat_upper(std: &StdForm, n_orig: usize, j: usize) -> Option<Affine1> {
    if std.ub[j] == f64::INFINITY {
        if j < n_orig {
            Some(Affine1::new(0.0, 1.0))
        } else {
            None
        }
    } else {
        Some(Affine1::new(std.ub[j], 0.0))
    }
}

/// 非基底状態 `status` にある列 `j` の値(`Lower`/`Upper` は [`ColCache`] の値、`Zero` は 0)。
/// その側が真の無限(スラックのみ)なら `None`。不変条件上起きないはずだが、起きた場合は
/// パニックではなく `?` で [`solve_slope_intercept_dual`] の `None`(`NotSolved`)まで伝播させる
/// (PyO3 境界でのプロセス中断を避けるため)。
#[inline]
fn nb_value_affine(cache: &ColCache, status: NbStatus, j: usize) -> Option<Affine1> {
    let r = match status {
        NbStatus::Lower => cache.lower[j],
        NbStatus::Upper => cache.upper[j],
        NbStatus::Zero => Some(Affine1::ZERO),
    };
    if r.is_none() && env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
        eprintln!("DEBUG_EXT_BAILOUT: nb_value_affine None at j={j} status={status:?}");
    }
    r
}

/// 現在の `basis_pos` から `B` を新たに LU 分解する(Forrest-Tomlin 更新ではない)。
/// 対角行列ならその専用分解、そうでなければ Markowitz 分解を行う。
///
/// `prev` は置き換える前の分解(あれば): そのピボット順を再利用する
/// (`sparse_lu::factorize_reusing`、HiGHS `HFactor::rebuild()` 相当)。再利用は各段で閾値
/// ピボットとフィルを再確認し、だめなら自動で完全な Markowitz 探索に戻るので、`prev` を渡しても
/// 分解が得られるかどうかは変わらず、手間だけが変わる。特異基底なら `None`。
fn refactorize(
    std: &StdForm,
    basis_pos: &[Option<usize>],
    prev: Option<&sparse_lu::FtLu>,
) -> Option<sparse_lu::FtLu> {
    let m = std.n_rows;
    // 行リストはスレッドローカルに再利用する(再確保せずクリアするので各行の容量が残る)。
    // 中身と順序は新規作成と同一。
    thread_local! {
        static ROWS: std::cell::RefCell<Vec<Vec<(usize, f64)>>> = const { std::cell::RefCell::new(Vec::new()) };
        /// 基底位置 → 変数番号の作業配列(同じくスレッドローカルに再利用)。
        static BASIS_OF: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    let mut rows = ROWS.with(|r| std::mem::take(&mut *r.borrow_mut()));
    rows.truncate(m);
    for row in rows.iter_mut() {
        row.clear();
    }
    rows.resize_with(m, Vec::new);
    // 基底位置 → 変数番号の逆引き(基底位置を持つ変数はちょうど `m` 個)。
    let mut basis_of = BASIS_OF.with(|b| std::mem::take(&mut *b.borrow_mut()));
    basis_of.clear();
    basis_of.resize(m, usize::MAX);
    for j in 0..std.n_total {
        if let Some(col) = basis_pos[j] {
            basis_of[col] = j;
        }
    }
    // `std.cols` による列駆動の構築(`nnz(A_B)` の手間)。基底位置 `col` の昇順に訪れるので
    // 各行の要素は列番号順に並び、`KernelMatrix::new` の安定ソートが並べ替えなしで済む
    // (整列済み入力なので分解結果は変数番号順に積んだ場合と同一)。
    for (col, &j) in basis_of.iter().enumerate() {
        if j == usize::MAX {
            continue;
        }
        for &(i, v) in std.cols.col(j) {
            rows[i].push((col, v));
        }
    }
    BASIS_OF.with(|b| *b.borrow_mut() = basis_of);
    let r = sparse_lu::factorize_diagonal(m, &rows)
        .map(sparse_lu::FtLu::new)
        .or_else(|| sparse_lu::factorize_reusing(m, &rows, prev));
    ROWS.with(|r| *r.borrow_mut() = rows);
    if r.is_none() {
        SINGULAR_BAILOUT.with(|f| f.set(true));
        if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
            eprintln!("DEBUG_EXT_BAILOUT: refactorize returned None (singular basis)");
        }
    }
    r
}

thread_local! {
    /// このスレッドの直近の求解で [`refactorize`] が特異基底を報告したか
    /// ([`solve_slope_intercept_dual`] が安全モードでの解き直しを判断するのに使う)。
    static SINGULAR_BAILOUT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 作業 #8 対処 5: このスレッドの直近の求解が、壊れた基底 (Farkas の証明の立たない実行不能の結論、対処 6 の `x_B` の不整合) か
    /// 反復上限で `None` を返したか (摂動を掛け直して解き直す)。
    static RESTART_BAILOUT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 作業 #5 M3: 主ループで確定した 1 ピボットの記録(巻き戻し用、[`rollback_pivots`])。
#[derive(Clone, Copy, Debug)]
struct PivotRec {
    /// 基底位置。
    r: usize,
    /// 入った列。
    q: usize,
    /// 出た列。
    leaving: usize,
    /// `q` の入基前の非基底状態。
    old_status_q: NbStatus,
    /// この反復までに記録した BFRT フリップ数(`pivot_flips` の長さ)。
    flips_end: usize,
}

/// 作業 #5 M1': 比率テストの候補 `cands` から、`|alpha_j| <= thr * ‖a_j‖∞` の雑音ピボットを外す
/// (順序は保つ)。`thr = C eps sqrt(w_r)`。残る候補数を返す(`dry` なら数えるだけで書き換えない)。
/// `col_inf_norm` は空なら作る(`‖a_j‖∞`、全列)。主ループのコード量を増やさないよう別関数にしている。
#[inline(never)]
fn filter_noise_pivots(std: &StdForm, col_inf_norm: &mut Vec<f64>, cands: &mut [Cand], thr: f64, dry: bool) -> usize {
    if col_inf_norm.is_empty() {
        col_inf_norm.extend((0..std.n_total).map(|j| std.cols.col(j).iter().fold(0.0f64, |a, &(_, v)| a.max(v.abs()))));
    }
    let mut w = 0usize;
    for idx in 0..cands.len() {
        let c = cands[idx];
        if c.hat_alpha.abs() > thr * col_inf_norm[c.j] {
            if !dry {
                cands[w] = c;
            }
            w += 1;
        }
    }
    w
}

/// polish 用: `active_cost` と分解 `lu` から被約費用 `d` を作り直す(`polish_with_true_bounds` の初期化と同じ計算)。
#[inline(never)]
fn polish_fresh_d(std: &StdForm, lu: &sparse_lu::FtLu, basis: &[usize], active_cost: &[f64], scratch: &mut [f64], d: &mut [f64]) {
    let m = std.n_rows;
    let c_b: Vec<f64> = basis.iter().map(|&bv| active_cost[bv]).collect();
    let mut y = vec![0.0f64; m];
    lu.solve_transpose_into(&c_b, scratch, &mut y);
    for j in 0..std.n_total {
        let mut dj = active_cost[j];
        for &(i, v) in std.cols.col(j) {
            dj -= v * y[i];
        }
        d[j] = dj;
    }
}

/// 作業 #5 M2: 行 `r` の BTRAN `rho = B^-T e_r` を Farkas の証明として、`A x = b`・`bound(j)` の
/// 境界で実行不能であることを確かめる(主ループの `Eligible = ∅`/BFRT 使い切り、polish の実行不能判定の直前)。
/// `g = rho^T A` と `t = rho^T b` について、境界上の `g^T x` の範囲 `[lo, hi]` が `t` を
/// 丸め誤差の見積もり(`100 eps` × 各項の大きさの和 + `1e-9 max(1, |t|)` + 0 とみなして捨てた項)を超えて外れていれば証明済み。基底列(`r` 以外)の `g_j` は
/// 理論上 0 で、`|g_j| <= 1e-9 max(Σ_i |rho_i a_ij|, ‖rho‖∞ ‖a_j‖∞)`(後退安定な LU の残差の水準)なら 0 とみなし、
/// 非基底列は比率テストと同じく `|g_j| <= TOL` を 0 とみなす。
/// LU が不安定(pilot87 を閾値 1e-3 で解くとき)や `x_B` が壊れているときの誤った `Infeasible` を防ぐ。
#[inline(never)]
fn infeasibility_certified(std: &StdForm, basis_pos: &[Option<usize>], nb_status: &[Option<NbStatus>], rho: &[f64], r: usize, bound: impl Fn(usize) -> (f64, f64)) -> bool {
    let m = std.n_rows;
    let n = std.n_total;
    let mut g = vec![0.0f64; n];
    let mut g_abs = vec![0.0f64; n];
    let mut t = 0.0f64;
    // `Σ_i |rho_i b_i|`(`t` の丸め誤差の尺度)。
    let mut scale = 0.0f64;
    for i in 0..m {
        let ri = rho[i];
        // PRICE と同じく `|rho_i| <= TOL` の行は使わない(証明に使う `y` を `rho` から作り直すだけなので、
        // `g` と `t` を同じ `y` から計算する限り証明としての正しさは変わらない)。
        if ri.abs() <= TOL {
            continue;
        }
        t += ri * std.b[i];
        scale += (ri * std.b[i]).abs();
        for &(j, v) in std.rows.row(i) {
            g[j] += ri * v;
            g_abs[j] += (ri * v).abs();
        }
    }
    let rho_max = rho.iter().fold(0.0f64, |a, &v| a.max(v.abs()));
    let debug = env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some();
    // `g^T x` の範囲 `[lo, hi]` と、それぞれの丸め誤差の見積もり(`err_lo`/`err_hi`): 計算した項の大きさの和の
    // `100 eps` 倍(`g_j` 自体の誤差 `eps Σ_i |rho_i a_ij|` を含む)と、0 とみなして捨てた項の有限な大きさ。
    let (mut lo, mut hi) = (0.0f64, 0.0f64);
    let (mut err_lo, mut err_hi) = (scale, scale);
    let mut drop_err = 0.0f64;
    // 診断: 各側に無限の寄与をした列の数 (基底 / 非基底) と先頭の例。
    let (mut inf_lo, mut inf_hi) = ([0usize; 2], [0usize; 2]);
    let mut examples: Vec<(usize, f64, f64, f64, Option<NbStatus>, f64)> = Vec::new();
    for j in 0..n {
        let mut gj = g[j];
        if gj == 0.0 {
            continue;
        }
        let (l, u) = bound(j);
        let dropped = if let Some(p) = basis_pos[j] {
            // 基底列(`r` 以外): 後退安定な LU の残差の水準なら 0 とみなす。
            let a_max = std.cols.col(j).iter().fold(0.0f64, |a, &(_, v)| a.max(v.abs()));
            p != r && gj.abs() <= 1e-9 * g_abs[j].max(rho_max * a_max)
        } else {
            // 非基底列: 比率テスト(`chuzc1_filter_one`)と同じく `|alpha_j| <= TOL` は 0 とみなす(M1' の雑音判定は
            // `rho` の誤差についての判定で、この `y` による証明の正しさとは関係しないので使わない)。
            gj.abs() <= TOL
        };
        if dropped {
            let (ea, eb) = ((gj * l).abs(), (gj * u).abs());
            let e = if ea.is_finite() && eb.is_finite() { ea.max(eb) } else if ea.is_finite() { ea } else if eb.is_finite() { eb } else { 0.0 };
            drop_err += e;
            gj = 0.0;
        }
        if gj == 0.0 {
            continue;
        }
        let (a, b) = if gj > 0.0 { (gj * l, gj * u) } else { (gj * u, gj * l) };
        lo += a;
        hi += b;
        let basic = basis_pos[j].is_some() as usize;
        if a.is_finite() {
            err_lo += g_abs[j] / gj.abs() * a.abs();
        } else {
            inf_lo[basic] += 1;
        }
        if b.is_finite() {
            err_hi += g_abs[j] / gj.abs() * b.abs();
        } else {
            inf_hi[basic] += 1;
        }
        if debug && !a.is_finite() && examples.len() < 4 {
            examples.push((j, gj, l, u, nb_status[j], g_abs[j]));
        }
    }
    let base_tol = 1e-9 * t.abs().max(1.0) + drop_err;
    let margin_lo = base_tol + 100.0 * f64::EPSILON * err_lo;
    let margin_hi = base_tol + 100.0 * f64::EPSILON * err_hi;
    let ok = t > hi + margin_hi || t < lo - margin_lo;
    if debug {
        eprintln!(
            "DEBUG_EXT: infeasibility certificate r={r} t={t:.6e} range=[{lo:.6e}, {hi:.6e}] margin=({margin_lo:.3e}, {margin_hi:.3e}) inf_lo(nb,basic)={inf_lo:?} inf_hi(nb,basic)={inf_hi:?} lo_examples={examples:?} certified={ok}"
        );
    }
    ok
}

/// [`infeasibility_certified`] 用: `M`-アフィン境界を実数に写す(傾きが非 0 なら傾きの符号の無限、無ければ `∓inf`)。
fn affine_bound_value(b: Option<Affine1>, missing: f64) -> f64 {
    match b {
        Some(a) if a.slope == 0.0 => a.base,
        Some(a) => a.slope.signum() * f64::INFINITY,
        None => missing,
    }
}

/// 作業 #5 M3: 比率テストの候補から、行 `r` について禁止された列(`bans` の `(r, q)`)を外す(順序は保つ)。
#[inline(never)]
fn filter_banned_pivots(r: usize, bans: &[(usize, usize)], cands: &mut [Cand]) -> usize {
    let mut w = 0usize;
    for idx in 0..cands.len() {
        let c = cands[idx];
        if !bans.contains(&(r, c.j)) {
            cands[w] = c;
            w += 1;
        }
    }
    w
}

/// 作業 #8 対処 3 (費用シフト): 非基底列 (固定列・`Zero` を除く) のうち、被約費用が双対実行可能側に摂動の大きさ
/// `s_j = (1 + r_j)(|c_j| + 1) base` 未満しか離れていない列の費用 `active_cost[j]` と `d[j]` を `s_j` だけ双対実行可能側へ
/// ずらす (`Lower` は `+`、`Upper` は `-`)。厳密な退化ピボットが続いたときだけ呼ぶ。ずらした列数を返す。
#[inline(never)]
fn shift_degenerate_costs(std: &StdForm, nb_status: &[Option<NbStatus>], base: f64, active_cost: &mut [f64], d: &mut [f64]) -> usize {
    let mut n_shifted = 0usize;
    for j in 0..std.n_total {
        let sign = match nb_status[j] {
            Some(NbStatus::Lower) => 1.0,
            Some(NbStatus::Upper) => -1.0,
            Some(NbStatus::Zero) | None => continue,
        };
        if std.lb[j] == std.ub[j] {
            continue;
        }
        let s = (1.0 + super::perturb_random(j)) * (std.c[j].abs() + 1.0) * base;
        if sign * d[j] < s {
            active_cost[j] += sign * s;
            d[j] += sign * s;
            n_shifted += 1;
        }
    }
    n_shifted
}

/// 作業 #8 対処 6 の主ループ側 (異常時だけ働くので主ループのコード量を増やさないよう別関数): ずれを診断に記録し、
/// 最初の求解 (`!safe_pivot`) で `tol` を超えたら解き直しを要求して真を返す。
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn xb_consistency_bailout(rho: &[f64], rows: Option<&[u32]>, rhs: &[f64], x_r: f64, tol: f64, safe_pivot: bool, debug: bool, iter: usize, r: usize, diag: &mut (f64, usize)) -> bool {
    let rel = xb_row_mismatch(rho, rows, rhs, x_r);
    if rel > diag.0 {
        *diag = (rel, iter);
    }
    if tol > 0.0 && rel > tol && !safe_pivot {
        RESTART_BAILOUT.with(|f| f.set(true));
        if debug {
            eprintln!("DEBUG_EXT_BAILOUT: x_B[{r}] inconsistent with rho^T(b - N x_N) (rel {rel:.3e}) at iter={iter} -> restart with re-perturbed costs");
        }
        return true;
    }
    false
}

/// 作業 #8 対処 6: `x_r`(FTRAN による `x_B[r]`)と `rho^T rhs`(`rho = B^-T e_r`、`rhs = b - N x_N`)のずれを、
/// 内積の項の大きさの和 `max(1, Σ |rho_i rhs_i|)` に対する比で返す。`rows` は `rho` の非ゼロ行の一覧 (PRICE の `rho_rows`、
/// `|rho_i| > TOL` の行) で、あればそれだけを走査する (大きな問題で再分解ごとの `O(m)` を避ける)。
#[inline(never)]
fn xb_row_mismatch(rho: &[f64], rows: Option<&[u32]>, rhs: &[f64], x_r: f64) -> f64 {
    let (mut t, mut scale) = (0.0f64, 0.0f64);
    let mut add = |i: usize| {
        let a = rho[i];
        if a != 0.0 {
            t += a * rhs[i];
            scale += (a * rhs[i]).abs();
        }
    };
    match rows {
        Some(rows) => rows.iter().for_each(|&i| add(i as usize)),
        None => (0..rho.len()).for_each(add),
    }
    (x_r - t).abs() / scale.max(1.0)
}

/// 作業 #5 M3: 再分解が特異(`refactorize` が `None`)だったとき、直近の成功した再分解以降に確定したピボット
/// (`log`)と BFRT フリップ(`flips`)を巻き戻して前の基底に戻し、分解し直す(主ループと polish で共有)。
/// (1) 最後の 1 ピボット(と後続のフリップ)だけを戻して試し、(2) まだ特異なら LU のピボット閾値を 1 段上げて
/// 同じ基底で試し(分解が不安定だった場合)、(3) それでもだめなら記録全体を戻す(記録の起点の基底は分解に
/// 成功している)、(4) 最後に前の分解のピボット順を使わずに分解し直す。1 ピボットを戻すたびに `on_undo` を呼ぶ(主ループの PRICE 行列の分割・`row_bounds` の復元用)。
/// `x_B`・`d`・実行不能行の集合は呼び出し側が再同期する(DSE 重みは戻さない)。成功すれば新しい分解と、
/// 以後禁止すべき `(r, q)` の一覧(戻したピボット、新しい順)を返す。記録が無いか戻しても特異なら `None`。
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn rollback_core(
    std: &StdForm,
    basis: &mut [usize],
    basis_pos: &mut [Option<usize>],
    nb_status: &mut [Option<NbStatus>],
    log: &mut Vec<PivotRec>,
    flips: &mut Vec<usize>,
    prev: &sparse_lu::FtLu,
    on_undo: &mut dyn FnMut(&PivotRec, &mut [Option<NbStatus>]),
) -> Option<(sparse_lu::FtLu, Vec<(usize, usize)>)> {
    let last = *log.last()?;
    let debug = env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some();
    // 戻したピボットの `(r, q)`(新しい順)。
    let mut undone: Vec<(usize, usize)> = Vec::new();
    // フリップを `keep` 個まで戻す(新しい順)。
    let undo_flips = |flips: &mut Vec<usize>, nb_status: &mut [Option<NbStatus>], keep: usize| {
        while flips.len() > keep {
            let j = flips.pop().unwrap();
            nb_status[j] = match nb_status[j] {
                Some(NbStatus::Lower) => Some(NbStatus::Upper),
                Some(NbStatus::Upper) => Some(NbStatus::Lower),
                s => s,
            };
        }
    };
    let mut n_undone = 0usize;
    for stage in 0..4 {
        if stage == 1 {
            // (2) 閾値を上げて同じ基底を分解し直す。上限なら省く。
            if !sparse_lu::escalate_pivot_threshold() {
                continue;
            }
        } else if stage == 3 {
            // (4) 記録の起点の基底を、前の分解のピボット順を使わずに分解し直す(数値的に特異に近い基底では
            // ピボット順で成否が分かれる)。
        } else {
            // (1) 最後の 1 ピボット、(3) 全体を戻す。
            let stop = if stage == 0 { log.len().saturating_sub(1) } else { 0 };
            if stage == 2 && log.is_empty() {
                // (1) で全部戻した(記録が 1 件だった)。(4) へ。
                continue;
            }
            while log.len() > stop {
                let rec = log.pop().unwrap();
                undo_flips(flips, nb_status, rec.flips_end);
                // 基底交換を戻す: `leaving` が位置 `r` に戻り、`q` は元の非基底状態へ。
                debug_assert_eq!(basis[rec.r], rec.q);
                basis[rec.r] = rec.leaving;
                basis_pos[rec.leaving] = Some(rec.r);
                nb_status[rec.leaving] = None;
                basis_pos[rec.q] = None;
                nb_status[rec.q] = Some(rec.old_status_q);
                on_undo(&rec, nb_status);
                undone.push((rec.r, rec.q));
                n_undone += 1;
            }
            if log.is_empty() {
                undo_flips(flips, nb_status, 0);
            }
        }
        let lu = refactorize(std, basis_pos, if stage == 3 { None } else { Some(prev) });
        if debug {
            eprintln!(
                "DEBUG_EXT: rollback stage={stage} undone={n_undone} threshold={:.3e} (last r={} q={}) refactor_ok={}",
                sparse_lu::pivot_threshold(),
                last.r,
                last.q,
                lu.is_some()
            );
        }
        if let Some(lu) = lu {
            // 禁止する `(r, q)`: (1)(2) で済めば最後の 1 ピボット、(3)(4) まで戻したなら戻した全ピボット
            // (最後の 1 つを戻しても特異だったので、原因はそれより前のピボットにある)。
            if stage <= 1 {
                undone.truncate(1);
            }
            return Some((lu, undone));
        }
    }
    None
}

/// 主ループ用の [`rollback_core`]: PRICE 行列の分割(非基底部/基底部)と `row_bounds`、`n_zero_nonbasic` も戻す。
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn rollback_pivots(
    std: &StdForm,
    cache: &ColCache,
    noise_feasible: &[bool],
    basis: &mut [usize],
    basis_pos: &mut [Option<usize>],
    nb_status: &mut [Option<NbStatus>],
    row_bounds: &mut RowBounds,
    log: &mut Vec<PivotRec>,
    flips: &mut Vec<usize>,
    price_nonbasic_only: bool,
    price_col: &mut [u32],
    price_val: &mut [f64],
    col_entry_of_price: &mut [u32],
    price_pos_of_col_entry: &mut [u32],
    price_nb_end: &mut [usize],
    col_entry_start: &[usize],
    price_start: &[usize],
    n_zero_nonbasic: &mut usize,
    prev: &sparse_lu::FtLu,
) -> Option<(sparse_lu::FtLu, Vec<(usize, usize)>)> {
    // PRICE 行列の 2 要素を入れ替える(主ループの `swap_entries` と同じ)。
    fn swap_entries(col: &mut [u32], val: &mut [f64], entry_of_price: &mut [u32], price_pos_of_col_entry: &mut [u32], a: usize, b: usize) {
        col.swap(a, b);
        val.swap(a, b);
        entry_of_price.swap(a, b);
        price_pos_of_col_entry[entry_of_price[a] as usize] = a as u32;
        price_pos_of_col_entry[entry_of_price[b] as usize] = b as u32;
    }
    let mut on_undo = |rec: &PivotRec, _nb: &mut [Option<NbStatus>]| {
        if rec.old_status_q == NbStatus::Zero {
            *n_zero_nonbasic += 1;
        }
        row_bounds.assign(rec.r, rec.leaving, cache, noise_feasible);
        if price_nonbasic_only {
            // 確定時と逆順: `leaving` を非基底部から基底部へ、`q` を基底部から非基底部へ。
            if std.lb[rec.leaving] != std.ub[rec.leaving] {
                for (k, &(i, _)) in std.cols.col(rec.leaving).iter().enumerate() {
                    let last_nb = price_nb_end[i] - 1;
                    let pos = price_pos_of_col_entry[col_entry_start[rec.leaving] + k] as usize;
                    debug_assert!(pos >= price_start[i] && pos <= last_nb);
                    swap_entries(price_col, price_val, col_entry_of_price, price_pos_of_col_entry, pos, last_nb);
                    price_nb_end[i] = last_nb;
                }
            }
            for (k, &(i, _)) in std.cols.col(rec.q).iter().enumerate() {
                let first = price_nb_end[i];
                let pos = price_pos_of_col_entry[col_entry_start[rec.q] + k] as usize;
                debug_assert!(pos >= first && pos < price_start[i + 1]);
                swap_entries(price_col, price_val, col_entry_of_price, price_pos_of_col_entry, pos, first);
                price_nb_end[i] = first + 1;
            }
        }
    };
    rollback_core(std, basis, basis_pos, nb_status, log, flips, prev, &mut on_undo)
}

/// `‖A_B x_B - rhs‖`(真の基底行列 `std.cols` の基底列と、解いた `x_b` による残差)。
/// `x_b` は基底位置順(変数順ではない)。FT 更新が受理され続けても LU の解が真の基底行列から
/// ずれていくドリフトを検出するのに使う(`polish_with_true_bounds` の残差トリガ)。
fn residual_norm(std: &StdForm, basis_pos: &[Option<usize>], x_b: &[f64], rhs: &[f64]) -> f64 {
    // `A_B x_B` を基底列ごとに累積する(`nnz(A_B)` の手間、`O(m)` のバッファ 1 本)。
    // `j` の昇順なので各行の加算順(したがって丸め)は行駆動と同一。
    let mut val = vec![0.0; std.n_rows];
    for j in 0..std.n_total {
        if let Some(pos) = basis_pos[j] {
            sparse_axpy_dense(x_b[pos], std.cols.col(j), &mut val);
        }
    }
    let mut resid_sq = 0.0f64;
    for i in 0..std.n_rows {
        let r = val[i] - rhs[i];
        resid_sq += r * r;
    }
    resid_sq.sqrt()
}

/// 策7 の chuzr 遅延最大ヒープの要素 (行 `row` の版 `ver` 時点のスコア)。順序は `Score2::cmp_lex`
/// (許容誤差 `LEX_REL_TOL`、短縮リストが有効なときの `score2_c2_tol` と同じ)、同点は行番号の小さい方が上。
/// 許容誤差は推移的でないのでヒープの根は近似的な最良だが、`BinaryHeap` は順序の矛盾で panic しない。
#[derive(Clone, Copy)]
struct ChuzrEntry {
    /// スコア。
    score: Score2,
    /// 行。
    row: u32,
    /// 積んだ時点の行の版番号 (`chuzr_ver[row]` と違えば古い)。
    ver: u32,
}

impl PartialEq for ChuzrEntry {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for ChuzrEntry {}

impl PartialOrd for ChuzrEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ChuzrEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score.cmp_lex(&other.score, LEX_REL_TOL).then_with(|| other.row.cmp(&self.row))
    }
}

/// chuzr 候補短縮リスト (S11・策7) の上位 `cap` 行ヒープに `(score, i)` を提示する。`heap` は
/// 「最も劣る行」を根に持つ二分ヒープ (順位は `Score2::cmp_lex` の降順、同点は行番号の昇順)。
/// 満杯なら根より良いときだけ根と入れ替える。1 行あたり `O(log cap)`。`cmp_lex` の許容誤差は推移的で
/// ないのでヒープの順序は近似だが、比較で panic することはない (集めた集合が上位 `cap` 行の近似になるだけ)。
fn shortlist_heap_offer(heap: &mut Vec<(Score2, usize)>, cap: usize, item: (Score2, usize), c2_tol: f64) {
    // `a` が `b` より上位か
    let before = |a: &(Score2, usize), b: &(Score2, usize)| match a.0.cmp_lex(&b.0, c2_tol) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => a.1 < b.1,
    };
    if heap.len() < cap {
        heap.push(item);
        // 上へ: 親より劣るなら入れ替える。
        let mut c = heap.len() - 1;
        while c > 0 {
            let p = (c - 1) / 2;
            if before(&heap[p], &heap[c]) {
                heap.swap(p, c);
                c = p;
            } else {
                break;
            }
        }
        return;
    }
    if !before(&item, &heap[0]) {
        return;
    }
    heap[0] = item;
    // 下へ: より劣る子と入れ替える。
    let n = heap.len();
    let mut p = 0;
    loop {
        let (l, r) = (2 * p + 1, 2 * p + 2);
        let mut worst = p;
        if l < n && before(&heap[worst], &heap[l]) {
            worst = l;
        }
        if r < n && before(&heap[worst], &heap[r]) {
            worst = r;
        }
        if worst == p {
            break;
        }
        heap.swap(p, worst);
        p = worst;
    }
}

/// [`residual_norm`] の 2 チャネル版: `x_B(M)` のドリフト検査用に、
/// `(‖A_B x_b_base - rhs_base‖, ‖A_B x_b_slope - rhs_slope‖)` を返す。基底列だけを
/// 1 回走査する(`nnz(A_B)` の手間)。`basis` は基底位置 → 変数番号。
/// `scratch_base`/`scratch_slope` は呼び出し側の長さ `m` の作業領域で、入口で全 0 であること。
/// 残差を集める最後の走査で 0 に戻して返す (入口の `fill` を省く。策9)。[`residual_scale_affine`]・
/// [`residual_norm_slope`] も同じ約束。
fn residual_norm_affine(
    std: &StdForm,
    basis: &[usize],
    x_b_base: &[f64],
    x_b_slope: &[f64],
    rhs_base: &[f64],
    rhs_slope: &[f64],
    scratch_base: &mut [f64],
    scratch_slope: &mut [f64],
) -> (f64, f64) {
    debug_assert!(scratch_base.iter().all(|&v| v == 0.0));
    // `rhs_slope` が空なのは `delta = 0` の合図で、そのとき `x_b_slope` も正確に 0 なので、
    // 傾きチャネルの残差は正確に `0.0`。計算せずに直接返す(全経路の値とビット同一)。
    if rhs_slope.is_empty() {
        let mut resid_base_sq = 0.0f64;
        for (pos, &j) in basis.iter().enumerate() {
            let b = x_b_base[pos];
            for &(i, v) in std.cols.col(j) {
                scratch_base[i] += v * b;
            }
        }
        for i in 0..std.n_rows {
            let rb = scratch_base[i] - rhs_base[i];
            scratch_base[i] = 0.0;
            resid_base_sq += rb * rb;
        }
        return (resid_base_sq.sqrt(), 0.0);
    }
    debug_assert!(scratch_slope.iter().all(|&v| v == 0.0));
    for (pos, &j) in basis.iter().enumerate() {
        let (b, s) = (x_b_base[pos], x_b_slope[pos]);
        if s == 0.0 {
            // `compute_rhs_affine` の傾き省略と同じ理由で正確な no-op: `+0.0` から始めた和は
            // `-0.0` にならないので、`v * 0.0 = ±0.0` を足しても何も変わらない。
            for &(i, v) in std.cols.col(j) {
                scratch_base[i] += v * b;
            }
            continue;
        }
        for &(i, v) in std.cols.col(j) {
            scratch_base[i] += v * b;
            scratch_slope[i] += v * s;
        }
    }
    let mut resid_base_sq = 0.0f64;
    let mut resid_slope_sq = 0.0f64;
    for i in 0..std.n_rows {
        let rb = scratch_base[i] - rhs_base[i];
        let rs = scratch_slope[i] - rhs_slope[i];
        scratch_base[i] = 0.0;
        scratch_slope[i] = 0.0;
        resid_base_sq += rb * rb;
        resid_slope_sq += rs * rs;
    }
    (resid_base_sq.sqrt(), resid_slope_sq.sqrt())
}

/// [`residual_norm_affine`] の残差の丸め誤差の尺度 `(‖|A_B||x_b_base| + |rhs_base|‖, 傾きチャネルの同じ量)`。
/// `A_B x_B` を浮動小数点で計算するだけで、残差には `ε × この量` 程度の誤差が乗る。
/// 引数と空の `rhs_slope`(`delta = 0`)の扱いは [`residual_norm_affine`] と同じ。
#[allow(clippy::too_many_arguments)]
fn residual_scale_affine(
    std: &StdForm,
    basis: &[usize],
    x_b_base: &[f64],
    x_b_slope: &[f64],
    rhs_base: &[f64],
    rhs_slope: &[f64],
    scratch_base: &mut [f64],
    scratch_slope: &mut [f64],
) -> (f64, f64) {
    let slope = !rhs_slope.is_empty();
    debug_assert!(scratch_base.iter().all(|&v| v == 0.0) && (!slope || scratch_slope.iter().all(|&v| v == 0.0)));
    for (pos, &j) in basis.iter().enumerate() {
        let (b, s) = (x_b_base[pos].abs(), if slope { x_b_slope[pos].abs() } else { 0.0 });
        for &(i, v) in std.cols.col(j) {
            scratch_base[i] += v.abs() * b;
            if slope {
                scratch_slope[i] += v.abs() * s;
            }
        }
    }
    let mut base_sq = 0.0f64;
    let mut slope_sq = 0.0f64;
    for i in 0..std.n_rows {
        let sb = scratch_base[i] + rhs_base.get(i).map_or(0.0, |r| r.abs());
        scratch_base[i] = 0.0;
        base_sq += sb * sb;
        if slope {
            let ss = scratch_slope[i] + rhs_slope[i].abs();
            scratch_slope[i] = 0.0;
            slope_sq += ss * ss;
        }
    }
    (base_sq.sqrt(), slope_sq.sqrt())
}

/// ドリフト検査の安価な事前検査 S2(`ENOMOTO_XB_DRIFT_SAMPLE`): 行 `i ≡ offset (mod k)` だけで
/// 残差 `A_B x_B - rhs` を `std.rows` から行方向に計算し(非基底要素は `basis_pos` で除外)、
/// 全体の 2 ノルムの推定値 `sqrt(Σ_sample r_i² · m / |sample|)` に拡大して
/// `(基底, 傾き)` で返す。手間は約 `nnz(A)/k`。`slope_only` は段階 A(基底チャネルは恒等的に 0)、
/// 空の `rhs_slope` は `delta = 0`(傾きチャネルは正確に 0)を表す。
#[allow(clippy::too_many_arguments)]
fn sampled_residual_affine(
    std: &StdForm,
    basis_pos: &[Option<usize>],
    x_b_base: &[f64],
    x_b_slope: &[f64],
    rhs_base: &[f64],
    rhs_slope: &[f64],
    slope_only: bool,
    k: usize,
    offset: usize,
) -> (f64, f64) {
    let m = std.n_rows;
    let do_slope = !rhs_slope.is_empty();
    let do_base = !slope_only;
    let mut sq_base = 0.0f64;
    let mut sq_slope = 0.0f64;
    let mut n = 0usize;
    let mut i = offset;
    while i < m {
        let mut sb = 0.0f64;
        let mut ss = 0.0f64;
        for &(j, v) in std.rows.row(i) {
            if let Some(p) = basis_pos[j] {
                sb += v * x_b_base[p];
                ss += v * x_b_slope[p];
            }
        }
        if do_base {
            let r = sb - rhs_base[i];
            sq_base += r * r;
        }
        if do_slope {
            let r = ss - rhs_slope[i];
            sq_slope += r * r;
        }
        n += 1;
        i += k;
    }
    if n == 0 {
        return (0.0, 0.0);
    }
    let scale = m as f64 / n as f64;
    ((sq_base * scale).sqrt(), (sq_slope * scale).sqrt())
}

/// `B x_B(M) = base + slope*M` の右辺(論文の補題 6.1 の `b - N x_N` を `M` に依存しない部分と
/// `M` の係数に分けたもの)を `(rhs_base, rhs_slope)` で返す。全非基底列を走査する `O(nnz(A))`。
/// 主ループは `x_B(M)` を増分維持するので、これは周期的なドリフト検査/再同期でだけ呼ぶ。
///
/// 返す `rhs_slope` は、人工 `M` 境界にある非基底列が 1 本も無い(`delta = 0`、論文の注意 6.11 の
/// 吸収状態 `x_N^1 = 0`)ときに限り**空**になる(長さ `m` の 0 を確保しない)。これが各利用者
/// ([`resolve_x_b_into`]・[`residual_norm_affine`]・`solve_x_b`)への「`M` 係数はもう無い」
/// という 1 語の合図になる。非基底列の値が得られなければ `None`。
fn compute_rhs_affine(std: &StdForm, cache: &ColCache, nb_status: &[Option<NbStatus>]) -> Option<(Vec<f64>, Vec<f64>)> {
    let m = std.n_rows;
    let mut rhs_base = std.b.clone();
    let mut rhs_slope: Vec<f64> = Vec::new();
    for j in 0..std.n_total {
        let Some(status) = nb_status[j] else { continue };
        let val = nb_value_affine(cache, status, j)?;
        if val.base == 0.0 && val.slope == 0.0 {
            continue;
        }
        if val.slope == 0.0 {
            // 有限側の列(大多数): 傾きチャネルへの寄与 `v * 0.0 = ±0.0` の減算は正確な no-op
            // (`+0.0` から始めた差分は丸めで `-0.0` にならない)なので省略してもビット同一。
            for &(i, v) in std.cols.col(j) {
                rhs_base[i] -= v * val.base;
            }
            continue;
        }
        // 傾きの寄与が初めて現れたときだけ傾きチャネルを確保する。
        if rhs_slope.is_empty() {
            rhs_slope = vec![0.0; m];
        }
        for &(i, v) in std.cols.col(j) {
            rhs_base[i] -= v * val.base;
            rhs_slope[i] -= v * val.slope;
        }
    }
    Some((rhs_base, rhs_slope))
}

/// [`compute_rhs_affine`] の出力に対する 2 回の FTRAN(`x_b_base = B^-1 rhs_base`、
/// `x_b_slope = B^-1 rhs_slope`)。論文の注意 6.11(`x_N^1 = 0` 以降は逸脱量が `M` に依存しない)を傾きチャネルに
/// 適用する: `rhs_slope` が空(`delta = 0`)なら `B^-1 rhs_slope` は恒等的に `±0.0` で、
/// `snap_slopes` 後は `fill(0.0)` とビット同一なので、求解を省いて合成クロックの tick だけを
/// 再現する([`sparse_lu::FtLu::add_zero_rhs_solve_ticks`]。CLOCK トリガとピボット経路は不変)。
/// この条件は再同期ごとに `nb_status` から判定し直す(ラッチではない)ので、`M` 境界が
/// 再び現れても求解が自動で再開される。`scratch` は長さ `m` の作業領域。
fn resolve_x_b_into(lu: &sparse_lu::FtLu, rhs_base: &[f64], rhs_slope: &[f64], scratch: &mut [f64], x_b_base: &mut [f64], x_b_slope: &mut [f64]) {
    lu.solve_into(rhs_base, scratch, x_b_base);
    if rhs_slope.is_empty() {
        lu.add_zero_rhs_solve_ticks(false);
        x_b_slope.fill(0.0);
    } else {
        lu.solve_into(rhs_slope, scratch, x_b_slope);
        snap_slopes(x_b_slope);
    }
}

/// 主ループが現在解いている問題(論文 7 節、Algorithm 1)。
/// `A` と `B` は 3 段階アルゴリズムの最初の 2 段階で、3 段階目(cleanup 補題の主押し出し)は
/// [`finish`]。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    /// 旧来の単一ループ形式: `M`-アフィン量を常に `(slope, base)` の組で比較する
    /// (`ENOMOTO_LEX_EXTENDED=1`、A/B 比較用のみ)。
    Lex,
    /// 段階 A、傾き問題(論文の式 (7)): 境界 `l^1, u^1` だけに対して `x^1` のみを維持する
    /// ([`ColCache::slope_problem`])。切片 `x^0` は段階 B への移行時に一度解くまで計算しない。
    A,
    /// 段階 B、切片問題(論文の式 (8)): `x^1` は固定(論文の命題 7.2 (ii))で、境界
    /// `l^B, u^B`([`ColCache::intercept_problem`])に対する通常の実数値双対単体法。
    B,
}

/// [`compute_rhs_affine`] の傾きチャネルだけ: 段階 A の右辺 `0 - N x_N^1`(論文の式 (7) の右辺は 0)。
/// 空なら恒等的に 0 という同じ約束に従う。
fn compute_rhs_slope_only(std: &StdForm, cache: &ColCache, nb_status: &[Option<NbStatus>]) -> Option<Vec<f64>> {
    let mut rhs_slope: Vec<f64> = Vec::new();
    for j in 0..std.n_total {
        let Some(status) = nb_status[j] else { continue };
        let val = nb_value_affine(cache, status, j)?;
        if val.slope == 0.0 {
            continue;
        }
        if rhs_slope.is_empty() {
            rhs_slope = vec![0.0; std.n_rows];
        }
        for &(i, v) in std.cols.col(j) {
            rhs_slope[i] -= v * val.slope;
        }
    }
    Some(rhs_slope)
}

/// 段階 A 用の [`resolve_x_b_into`]: 傾きチャネルだけを解き、`x_b_base` は段階 A の値である
/// 正確な 0 にする(FTRAN しない)。
fn resolve_x_b_slope_only(lu: &sparse_lu::FtLu, rhs_slope: &[f64], scratch: &mut [f64], x_b_base: &mut [f64], x_b_slope: &mut [f64]) {
    x_b_base.fill(0.0);
    if rhs_slope.is_empty() {
        lu.add_zero_rhs_solve_ticks(false);
        x_b_slope.fill(0.0);
    } else {
        lu.solve_into(rhs_slope, scratch, x_b_slope);
        snap_slopes(x_b_slope);
    }
}

/// 現在の段階について `nb_status` から `x_B(M)` を完全に再同期する: 新しい右辺の組
/// (ドリフト検査の `rhs_inc_*` の基準)を返し、`x_b_base`/`x_b_slope` に解を書く。
/// 段階 A は傾きチャネルだけを解き、基底側は全 0 を返す。段階 B の境界は傾きを持たないので
/// 傾きチャネルは空で返り、[`resolve_x_b_into`] 自身が求解を省く。`scratch` は作業領域。
#[allow(clippy::too_many_arguments)]
fn resync_x_b(
    std: &StdForm,
    cache: &ColCache,
    nb_status: &[Option<NbStatus>],
    lu: &sparse_lu::FtLu,
    phase: Phase,
    scratch: &mut [f64],
    x_b_base: &mut [f64],
    x_b_slope: &mut [f64],
) -> Option<(Vec<f64>, Vec<f64>)> {
    if phase == Phase::A {
        let rhs_slope = compute_rhs_slope_only(std, cache, nb_status)?;
        resolve_x_b_slope_only(lu, &rhs_slope, scratch, x_b_base, x_b_slope);
        Some((vec![0.0; std.n_rows], rhs_slope))
    } else {
        let (rhs_base, rhs_slope) = compute_rhs_affine(std, cache, nb_status)?;
        resolve_x_b_into(lu, &rhs_base, &rhs_slope, scratch, x_b_base, x_b_slope);
        Some((rhs_base, rhs_slope))
    }
}

/// [`residual_norm_affine`] の傾きチャネルだけ(段階 A のドリフト検査用。空の `rhs_slope` は恒等的に 0)。
fn residual_norm_slope(std: &StdForm, basis: &[usize], x_b_slope: &[f64], rhs_slope: &[f64], scratch: &mut [f64]) -> f64 {
    if rhs_slope.is_empty() {
        return 0.0;
    }
    debug_assert!(scratch.iter().all(|&v| v == 0.0));
    for (pos, &j) in basis.iter().enumerate() {
        let s = x_b_slope[pos];
        if s == 0.0 {
            continue;
        }
        for &(i, v) in std.cols.col(j) {
            scratch[i] += v * s;
        }
    }
    let mut resid_sq = 0.0f64;
    for i in 0..std.n_rows {
        let r = scratch[i] - rhs_slope[i];
        scratch[i] = 0.0;
        resid_sq += r * r;
    }
    resid_sq.sqrt()
}

/// `x_B(M) = base + slope*M`(論文の補題 6.1)を同じ分解に対する 2 回の独立な FTRAN で求める
/// (記号的な線形代数ではなく、通常の数値求解を 2 回行うだけ)。都度確保する一回限りの版で、
/// `finish` から呼ぶ。
fn solve_x_b(std: &StdForm, lu: &sparse_lu::FtLu, nb_status: &[Option<NbStatus>], cache: &ColCache) -> Option<(Vec<f64>, Vec<f64>)> {
    let (rhs_base, rhs_slope) = compute_rhs_affine(std, cache, nb_status)?;
    // [`resolve_x_b_into`] と同じく、正確に 0 になる傾きの求解を省いて tick だけを再現する
    // (この `lu` は polish に引き継がれ、その CLOCK トリガが合成クロックを読むため)。
    let x_b_slope = if rhs_slope.is_empty() {
        lu.add_zero_rhs_solve_ticks(false);
        vec![0.0; std.n_rows]
    } else {
        lu.solve(&rhs_slope)
    };
    Some((lu.solve(&rhs_base), x_b_slope))
}

/// 基底位置 `i` の変数の境界からの逸脱を `(d_dir, dev)` で返す。下限未満(増やしたい)なら
/// `d_dir == 1`、上限超過(減らしたい)なら `-1`、実行可能なら `None`。chuzr の走査と
/// [`row_infeasible_affine`] が同じ実装を共有するためにまとめてある。
#[inline]
fn row_deviation(cache: &ColCache, basis: &[usize], x_b_base: &[f64], x_b_slope: &[f64], noise_feasible: &[bool], i: usize) -> Option<(i32, Affine1)> {
    let bv = basis[i];
    // Eligible 空の時点で「丸め誤差程度の実行不能」と判定済みの行(`noise_feasible`)は、
    // 以後実行可能として扱う(毎反復選び直して chuzc1 で失敗し続けないため)。
    deviation_core(noise_feasible[bv], cache.lower[bv], cache.upper[bv], x_b_base[i], x_b_slope[i])
}

/// [`row_deviation`] の計算本体。基底変数の `noise_feasible` フラグと境界 `lower`/`upper`、
/// 値 `x_base + x_slope*M` を直接受け取る([`RowBounds::deviation`] と共有するため)。
/// 両側で違反していれば辞書式に大きい方を返す。
#[inline(always)]
fn deviation_core(noise: bool, lower: Option<Affine1>, upper: Option<Affine1>, x_base: f64, x_slope: f64) -> Option<(i32, Affine1)> {
    if noise {
        return None;
    }
    let x_bi = Affine1::new(x_base, x_slope);
    // `lower` が `None` なのは下限が真の(`M` でない)無限のとき(有限の値では決して違反しない)。
    // `upper` も同様。
    let dev_minus = lower.and_then(|hat_l| {
        let v_minus = hat_l.sub(x_bi);
        v_minus.gt_zero().then_some(v_minus)
    });
    let dev_plus = upper.and_then(|hat_u| {
        let v_plus = x_bi.sub(hat_u);
        v_plus.gt_zero().then_some(v_plus)
    });
    match (dev_minus, dev_plus) {
        (Some(vm), Some(vp)) => Some(if vm.cmp_lex(&vp) == std::cmp::Ordering::Greater { (1i32, vm) } else { (-1i32, vp) }),
        (Some(vm), None) => Some((1i32, vm)),
        (None, Some(vp)) => Some((-1i32, vp)),
        (None, None) => None,
    }
}

/// [`row_deviation`] を [`InfeasibleRows`] 用の真偽値にしたもの(行 `i` が実行不能か)。
#[inline]
#[cfg_attr(not(debug_assertions), allow(dead_code))]
fn row_infeasible_affine(cache: &ColCache, basis: &[usize], x_b_base: &[f64], x_b_slope: &[f64], noise_feasible: &[bool], i: usize) -> bool {
    row_deviation(cache, basis, x_b_base, x_b_slope, noise_feasible, i).is_some()
}

/// [`InfeasibleRows`] に現在含まれる各行について、そのメンバーシップを最後に判定した時点の
/// [`row_deviation`] の結果のキャッシュ(HiGHS の `work_infeasibility` 相当)。`row_deviation` が
/// 読む入力(`x_b_base[i]`、`x_b_slope[i]`、`basis[i]`、`noise_feasible[basis[i]]`)は行 `i` の
/// メンバーシップを判定し直す箇所([`refresh_row`]/[`rebuild_rows`])でしか変わらないので、
/// プール内の行については新たに計算した値とビット単位で一致する。プール外の行の値は古く、読まない。
struct RowDevCache {
    /// 逸脱の方向(`1`: 下限未満、`-1`: 上限超過)。
    dir: Vec<i32>,
    /// 逸脱量(`M` のアフィン関数)。
    dev: Vec<Affine1>,
}

impl RowDevCache {
    /// 長さ `m` のキャッシュを作る(中身は未使用値)。
    fn new(m: usize) -> Self {
        RowDevCache { dir: vec![0; m], dev: vec![Affine1::ZERO; m] }
    }
}

/// 現在の基底変数の境界([`ColCache`])と `noise_feasible` フラグを、変数ではなく基底**行**で
/// 添字付けしたもの(HiGHS の `baseLower`/`baseUpper` 相当)。`x_B` 更新後の実行可能性再判定
/// ([`refresh_row`])は行を昇順に走査するので、`basis[i]` 経由のランダムアクセスを連続アクセスに
/// する。`basis[i]` の書き換えのたびに [`RowBounds::assign`]、基底変数の `noise_feasible` 書き換えの
/// たびに [`RowBounds::mark_noise`] で更新しなければならない。[`RowBounds::deviation`] は
/// [`row_deviation`] とビット単位で一致する(`debug_assertions` で検査)。
struct RowBounds {
    /// 行 `i` の基底変数の下側境界。
    lower: Vec<Option<Affine1>>,
    /// 行 `i` の基底変数の上側境界。
    upper: Vec<Option<Affine1>>,
    /// [`deviation_flat`] 用の `(lower, upper)` の `f64` 版: 実の有限境界(傾き 0)ならその値、
    /// 無いか人工 `∓M` 側なら `∓inf`。`noise` の行は `(-inf, +inf)` として格納するので、
    /// 平坦経路は 1 行あたり 16 バイトの組だけを読めばよい(S10)。
    flat: Vec<[f64; 2]>,
    /// 行 `i` の基底変数が `noise_feasible` か。
    noise: Vec<bool>,
}

/// [`RowBounds::flat`] の 1 行分を作る(`noise` なら `(-inf, +inf)`)。
#[inline]
fn flat_pair(lower: Option<Affine1>, upper: Option<Affine1>, noise: bool) -> [f64; 2] {
    if noise {
        [f64::NEG_INFINITY, f64::INFINITY]
    } else {
        [flat_bound(lower, f64::NEG_INFINITY), flat_bound(upper, f64::INFINITY)]
    }
}

/// 境界 `b` が実の有限境界(傾き 0)ならその値、そうでなければ `missing`(`∓inf`)を返す。
#[inline]
fn flat_bound(b: Option<Affine1>, missing: f64) -> f64 {
    match b {
        Some(a) if a.slope == 0.0 => a.base,
        _ => missing,
    }
}

impl RowBounds {
    /// 現在の基底 `basis` から全行分を作る。
    fn new(cache: &ColCache, basis: &[usize], noise_feasible: &[bool]) -> Self {
        RowBounds {
            lower: basis.iter().map(|&j| cache.lower[j]).collect(),
            upper: basis.iter().map(|&j| cache.upper[j]).collect(),
            flat: basis.iter().map(|&j| flat_pair(cache.lower[j], cache.upper[j], noise_feasible[j])).collect(),
            noise: basis.iter().map(|&j| noise_feasible[j]).collect(),
        }
    }

    /// 行 `i` の基底変数が `j` になった(境界を差し替える)。
    #[inline]
    fn assign(&mut self, i: usize, j: usize, cache: &ColCache, noise_feasible: &[bool]) {
        self.lower[i] = cache.lower[j];
        self.upper[i] = cache.upper[j];
        self.flat[i] = flat_pair(cache.lower[j], cache.upper[j], noise_feasible[j]);
        self.noise[i] = noise_feasible[j];
    }

    /// 行 `i` の現在の基底変数が `noise_feasible` になった。
    #[inline]
    fn mark_noise(&mut self, i: usize) {
        self.noise[i] = true;
        self.flat[i] = [f64::NEG_INFINITY, f64::INFINITY];
    }

    /// 行 `i` の逸脱([`row_deviation`] と同一)。傾きが 0 なら平坦経路 [`deviation_flat`] を使う。
    #[inline]
    fn deviation(&self, x_b_base: &[f64], x_b_slope: &[f64], i: usize) -> Option<(i32, Affine1)> {
        let xs = x_b_slope[i];
        if xs == 0.0 {
            let [lo, hi] = self.flat[i];
            deviation_flat(false, lo, hi, x_b_base[i], xs)
        } else {
            deviation_core(self.noise[i], self.lower[i], self.upper[i], x_b_base[i], xs)
        }
    }
}

/// `x_B` の傾きがちょうど 0 の行についての [`deviation_core`](delta=0 以降のほぼ全行・全反復)。
/// 傾き 0 では人工 `∓M` 境界の逸脱は傾き `-1` で決して `≻ 0` にならないので、無い境界
/// (`∓inf`)と同じに扱え、実の境界の逸脱は `M` を含まないので [`Affine1::gt_zero`] は
/// `LEX_REL_TOL < base < inf` に帰着する。[`deviation_core`] とビット単位で同じ結果
/// (同じ `base` 計算・同じ `slope` 式・同じタイブレーク)で、`Affine1`/`Option` の手間だけを省く。
/// `lower`/`upper` は [`RowBounds::flat`] の値、`x`/`xs` は基底値と傾き。
#[inline(always)]
fn deviation_flat(noise: bool, lower: f64, upper: f64, x: f64, xs: f64) -> Option<(i32, Affine1)> {
    if noise {
        return None;
    }
    let vm = lower - x;
    let vp = x - upper;
    let m_ok = vm > LEX_REL_TOL && vm < f64::INFINITY;
    let p_ok = vp > LEX_REL_TOL && vp < f64::INFINITY;
    let dev_minus = Affine1::new(vm, 0.0 - xs);
    let dev_plus = Affine1::new(vp, xs - 0.0);
    match (m_ok, p_ok) {
        (true, true) => Some(if dev_minus.cmp_lex(&dev_plus) == std::cmp::Ordering::Greater { (1i32, dev_minus) } else { (-1i32, dev_plus) }),
        (true, false) => Some((1i32, dev_minus)),
        (false, true) => Some((-1i32, dev_plus)),
        (false, false) => None,
    }
}

/// 行 `i` の [`InfeasibleRows`] メンバーシップを判定し直し、逸脱自体も `dev_cache`([`RowDevCache`])に
/// キャッシュする。`row_bounds` は行ごとの境界キャッシュ。
#[inline]
#[allow(clippy::too_many_arguments)]
fn refresh_row(rows: &mut InfeasibleRows, dev_cache: &mut RowDevCache, row_bounds: &RowBounds, x_b_base: &[f64], x_b_slope: &[f64], i: usize) {
    // S10 の高速経路: 傾き 0 の行(ほぼ全行)が実行可能([`deviation_flat`] の 2 判定がともに偽)
    // なら、結果の組を作らずメンバーシップを外すだけ。同じ値に同じ判定なのでビット同一。
    if x_b_slope[i] == 0.0 {
        let [lo, hi] = row_bounds.flat[i];
        let x = x_b_base[i];
        let vm = lo - x;
        let vp = x - hi;
        if !(vm > LEX_REL_TOL && vm < f64::INFINITY) && !(vp > LEX_REL_TOL && vp < f64::INFINITY) {
            rows.set(i, false);
            return;
        }
    }
    match row_bounds.deviation(x_b_base, x_b_slope, i) {
        Some((dir, dev)) => {
            dev_cache.dir[i] = dir;
            dev_cache.dev[i] = dev;
            rows.set(i, true);
        }
        None => rows.set(i, false),
    }
}

/// [`refresh_row`] の全行再構築版(`InfeasibleRows::rebuild`)。
#[allow(clippy::too_many_arguments)]
fn rebuild_rows(rows: &mut InfeasibleRows, dev_cache: &mut RowDevCache, m: usize, row_bounds: &RowBounds, x_b_base: &[f64], x_b_slope: &[f64]) {
    rows.rebuild(m, |i| match row_bounds.deviation(x_b_base, x_b_slope, i) {
        Some((dir, dev)) => {
            dev_cache.dir[i] = dir;
            dev_cache.dev[i] = dev;
            true
        }
        None => false,
    });
}

/// [`row_deviation`] の `f64` 版([`polish_with_true_bounds`] 用)。cleanup 後は `M` フラグ付きの
/// 非基底列が無いので、`std.lb`/`std.ub` を直接使う単一チャネルで判定する。
///
/// 基底位置 `i` の変数が下限未満なら `(1, lb - x)`、上限超過なら `(-1, x - ub)`、
/// 実行可能(または `noise_feasible`)なら `None`。許容誤差は行の値で尺度付けした
/// `PRIMAL_FEAS_TOL * max(|x_i|, 1)`。
fn row_deviation_plain(std: &StdForm, basis: &[usize], x_b: &[f64], noise_feasible: &[bool], i: usize) -> Option<(i32, f64)> {
    let bv = basis[i];
    if noise_feasible[bv] {
        return None;
    }
    let xi = x_b[i];
    let feas_tol = tunable!("ENOMOTO_T_PRIMAL_FEAS_TOL", PRIMAL_FEAS_TOL, f64) * xi.abs().max(1.0);
    if xi < std.lb[bv] - feas_tol {
        Some((1, std.lb[bv] - xi))
    } else if xi > std.ub[bv] + feas_tol {
        Some((-1, xi - std.ub[bv]))
    } else {
        None
    }
}

/// [`row_deviation_plain`] を [`InfeasibleRows`] 用の真偽値にしたもの(行 `i` が実行不能か)。
fn row_infeasible_plain(std: &StdForm, basis: &[usize], x_b: &[f64], noise_feasible: &[bool], i: usize) -> bool {
    row_deviation_plain(std, basis, x_b, noise_feasible, i).is_some()
}

/// `b - N x_N` の `f64` 版([`compute_rhs_affine`] の単一チャネル版)。
/// [`polish_with_true_bounds`] の初期化と周期的な再同期でだけ呼ぶ。cleanup 後は全非基底列が
/// 有限な境界値にある(`Zero` は値 0 なので寄与しない)。
fn compute_rhs_plain(std: &StdForm, nb_status: &[Option<NbStatus>]) -> Vec<f64> {
    let mut rhs = std.b.clone();
    for j in 0..std.n_total {
        let Some(status) = nb_status[j] else { continue };
        let val = match status {
            NbStatus::Lower => std.lb[j],
            NbStatus::Upper => std.ub[j],
            NbStatus::Zero => continue,
        };
        if val == 0.0 {
            continue;
        }
        for &(i, v) in std.cols.col(j) {
            rhs[i] -= v * val;
        }
    }
    rhs
}

/// 論文の `sigma_j`(3.1 節 (ii)): 非基底状態 `status` の列の向き。`Lower` なら `+1`、`Upper` なら
/// `-1`、自由列の `Zero` なら `d_dir * sigma * alpha_j < 0` となる符号(離基行が必要とする
/// 方向にいつでも適格。`alpha_j != 0` の判定は呼び出し側が行う)。
/// `d_dir` は離基行の方向、`alpha_j` はその列のピボット行要素。
#[inline(always)]
fn nb_sigma(status: NbStatus, d_dir: f64, alpha_j: f64) -> f64 {
    match status {
        NbStatus::Lower => 1.0,
        NbStatus::Upper => -1.0,
        NbStatus::Zero => {
            if d_dir * alpha_j > 0.0 {
                -1.0
            } else {
                1.0
            }
        }
    }
}

/// 列 `j` の幅 `ub[j] - lb[j]`(`f64` 版、[`polish_with_true_bounds`] 用)。
/// 片側無限のスラックでは `f64::INFINITY` になる(呼び出し側が `is_finite()` で判定)。
fn width_plain(std: &StdForm, j: usize) -> f64 {
    std.ub[j] - std.lb[j]
}

/// 被約費用 `d = c - A^T B^{-T} c_B` を現在の分解から新たに計算し直す(`super::fresh_d` の
/// 拡張版。基底列は 0)。開始時と再分解のたびに呼び、増分更新 `d[j] -= theta_d * a_p[j]` が
/// 蓄積する誤差をリセットする。
///
/// 引数: `active_cost` は摂動済みコスト、`cb_buf`/`scratch`/`y_buf` は長さ `m` の作業領域
/// (`y_buf` には `y = B^-T c_B` が残る)、`d` は出力(長さ `n_total`)。
fn fresh_d_into(std: &StdForm, lu: &sparse_lu::FtLu, basis: &[usize], basis_pos: &[Option<usize>], active_cost: &[f64], cb_buf: &mut [f64], scratch: &mut [f64], y_buf: &mut [f64], d: &mut [f64]) {
    for i in 0..std.n_rows {
        cb_buf[i] = active_cost[basis[i]];
    }
    lu.solve_transpose_into(cb_buf, scratch, y_buf);
    for j in 0..std.n_total {
        if basis_pos[j].is_some() {
            d[j] = 0.0;
            continue;
        }
        let mut dj = active_cost[j];
        for &(i, v) in std.cols.col(j) {
            dj -= v * y_buf[i];
        }
        d[j] = dj;
    }
}

/// 全基底コストが正確に `+0.0`(全スラック開始時)の場合の [`fresh_d_into`](S14)。BTRAN と
/// `O(nnz(A))` の走査を省く。前提が成り立たなければ何も触らず `false` を返す。
///
/// ビット同一性: `c_B` がすべて `+0.0` なら BTRAN の各段は 0 しか見ないので `scratch`/`y_buf` は
/// すべて `+0.0` になり(ここで直接書く)、[`sparse_lu::FtLu::add_zero_rhs_btran_ticks`] で
/// 同じ合成クロック tick を加える。非基底列の `d[j] = c_j - Σ v·(+0.0)` は `c_j` に一致するが、
/// `c_j = -0.0` のときだけ結果の符号が要素の符号に依存するので、その(まれな)列は
/// [`fresh_d_into`] と同じループで計算する。
fn fresh_d_into_zero_y(std: &StdForm, lu: &sparse_lu::FtLu, basis: &[usize], basis_pos: &[Option<usize>], active_cost: &[f64], cb_buf: &mut [f64], scratch: &mut [f64], y_buf: &mut [f64], d: &mut [f64]) -> bool {
    if !basis.iter().all(|&bv| active_cost[bv].to_bits() == 0) {
        return false;
    }
    cb_buf.iter_mut().for_each(|v| *v = 0.0);
    scratch.iter_mut().for_each(|v| *v = 0.0);
    y_buf.iter_mut().for_each(|v| *v = 0.0);
    lu.add_zero_rhs_btran_ticks();
    // `-0.0` のビットパターン。
    const NEG_ZERO: u64 = 0x8000_0000_0000_0000;
    for j in 0..std.n_total {
        if basis_pos[j].is_some() {
            d[j] = 0.0;
            continue;
        }
        let cj = active_cost[j];
        if cj.to_bits() == NEG_ZERO {
            let mut dj = cj;
            for &(i, v) in std.cols.col(j) {
                dj -= v * y_buf[i];
            }
            d[j] = dj;
        } else {
            d[j] = cj;
        }
    }
    true
}

/// コスト符号による双対実行可能な crash(論文の命題 5.1): 各構造列を、コスト `cost[j]` が非負なら
/// `Lower`、負なら `Upper` に置く(スラック列は基底なので `None`)。[`hat_lower`]/[`hat_upper`]
/// が全構造列で定義されるので、両側の境界が有限である必要は無い。
///
/// `cost` は摂動済みコスト(`super::perturb_costs` の出力)を受け取る(`d` の増分維持と
/// 同じコストを使って整合を保つため)。コスト 0 の自由列(`lb == -inf` かつ `ub == +inf`)は
/// `Zero`(値 0)に置く(論文の式 (4) の第 3 の場合。`Lower`/`Upper` だと `-M`/`+M`
/// になる)。`perturb_costs` は自由列を摂動しないので `cost[j]` は真のコスト。
fn crash(std: &StdForm, cost: &[f64], n_orig: usize) -> Vec<Option<NbStatus>> {
    let mut nb_status = vec![None; std.n_total];
    for j in 0..n_orig {
        let free = std.lb[j] == f64::NEG_INFINITY && std.ub[j] == f64::INFINITY;
        nb_status[j] = Some(if free && cost[j].abs() <= TOL {
            NbStatus::Zero
        } else if cost[j] >= -TOL {
            NbStatus::Lower
        } else {
            NbStatus::Upper
        });
    }
    nb_status
}

/// 実験的な crash の改良(`ENOMOTO_CRASH_ZERO_COST_PLACEMENT`、既定オフ・効果なしと確認済み):
/// 真のコストがちょうど 0 で両境界が有限(箱型)の非基底列は、全スラック基底でどちらの境界に
/// 置いても被約費用が 0 なので、列番号順に一度だけ走査し、触れる行の違反量の和(この時点の
/// 行残差 `b - Σ 非基底列の寄与` に対して)が小さくなる側に置く。
///
/// `Upper` に移した列は `active_cost[j]` の符号も反転し(摂動が逆側に出た場合と同じ)、`d` の
/// 符号の前提(`Upper` なら `d[j] <= 0`)を保つ。引数 `nb_status` と `active_cost` を書き換える。
fn refine_zero_cost_placement(std: &StdForm, active_cost: &mut [f64], nb_status: &mut [Option<NbStatus>], n_orig: usize) {
    let n_rows = std.n_rows;
    // 行残差 `b - Σ 非基底列の寄与`(全スラック基底では `x_B[i] = residual[i]`)。
    let mut residual = std.b.clone();
    // 置き場所を選び直す箱型・コスト 0 の列。
    let mut flexible: Vec<usize> = Vec::new();
    for j in 0..n_orig {
        let lo = std.lb[j];
        let hi = std.ub[j];
        if lo == hi {
            continue; // 固定列: どちらでも寄与は同じなので完全に除外する
        }
        if std.c[j] == 0.0 && lo.is_finite() && hi.is_finite() {
            flexible.push(j);
            continue; // 下の走査で決めるので、まだ残差から引かない
        }
        let x = match nb_status[j] {
            Some(NbStatus::Lower) => lo,
            Some(NbStatus::Upper) => hi,
            Some(NbStatus::Zero) | None => continue,
        };
        if x != 0.0 {
            for &(i, a) in std.cols.col(j) {
                residual[i] -= a * x;
            }
        }
    }
    if flexible.is_empty() {
        return;
    }
    // 診断(`ENOMOTO_DEBUG_EXT_CRASH`)を出力するか。
    let debug = env_str!("ENOMOTO_DEBUG_EXT_CRASH").is_some();
    // 値 `v` の区間 `[lo, hi]` からの違反量。
    let violation = |v: f64, lo: f64, hi: f64| -> f64 {
        if v < lo { lo - v } else if v > hi { v - hi } else { 0.0 }
    };
    // 行残差に対する(スラック境界に照らした)実行不能行の数。
    let infeasible_count = |residual: &[f64]| -> usize {
        (0..n_rows)
            .filter(|&i| violation(residual[i], std.lb[n_orig + i], std.ub[n_orig + i]) > TOL)
            .count()
    };
    // 診断出力用の比較基準: crash のとおり全箱型列を `Lower` に置いたときの実行不能行数。
    let before = if debug {
        let mut baseline = residual.clone();
        for &j in &flexible {
            let lo = std.lb[j];
            if lo != 0.0 {
                for &(i, a) in std.cols.col(j) {
                    baseline[i] -= a * lo;
                }
            }
        }
        Some(infeasible_count(&baseline))
    } else {
        None
    };
    // 箱型列の数と、`Upper` に移した列の数(診断用)。
    let n_flexible = flexible.len();
    let mut flipped = 0usize;
    for j in flexible {
        let lo = std.lb[j];
        let hi = std.ub[j];
        let col = std.cols.col(j);
        let mut viol_lo = 0.0f64;
        let mut viol_hi = 0.0f64;
        for &(i, a) in col {
            let bi_lo = std.lb[n_orig + i];
            let bi_hi = std.ub[n_orig + i];
            viol_lo += violation(residual[i] - a * lo, bi_lo, bi_hi);
            viol_hi += violation(residual[i] - a * hi, bi_lo, bi_hi);
        }
        let (status, x) = if viol_hi < viol_lo { (NbStatus::Upper, hi) } else { (NbStatus::Lower, lo) };
        if matches!(status, NbStatus::Upper) {
            flipped += 1;
            active_cost[j] = -active_cost[j];
        }
        nb_status[j] = Some(status);
        if x != 0.0 {
            for &(i, a) in col {
                residual[i] -= a * x;
            }
        }
    }
    if debug {
        eprintln!(
            "DEBUG_EXT_CRASH: flexible_cols={n_flexible} flipped_to_upper={flipped} infeasible_rows_before={} infeasible_rows_after={}",
            before.unwrap(),
            infeasible_count(&residual)
        );
    }
}

/// 入る列の比率テスト/BFRT 歩進の候補 1 つ。
#[derive(Clone, Copy)]
struct Cand {
    /// 列番号。
    j: usize,
    /// 符号付きピボット行要素 `sigma_j * alpha_j`。
    hat_alpha: f64,
    /// 比 `hat_c_j / |hat_alpha_j|`(双対比率テストの比)。
    ratio: f64,
}

// `(ratio, j)` の昇順(`j` で同値を解消して挿入順に依存しない)。`BinaryHeap<Reverse<Cand>>`
// で chuzc1/BFRT の歩進がこの順を遅延的に取り出せるようにする。
impl PartialEq for Cand {
    fn eq(&self, other: &Self) -> bool {
        self.ratio == other.ratio && self.j == other.j
    }
}
impl Eq for Cand {}
impl PartialOrd for Cand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Cand {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ratio.total_cmp(&other.ratio).then_with(|| self.j.cmp(&other.j))
    }
}

/// 離基行候補 `row` についての試行 BTRAN + PRICE +(BFRT なしの)比率テスト。基底や非基底状態は
/// 一切変更しない(「最大改善」chuzr エスカレーションの部品)。行 `row` の Eligible 集合での
/// 最小比 `hat_c_j / |hat_alpha_j|` を返し、Eligible が空なら `None`。呼び出し側は
/// `dev_i.scale(ratio)` をその行の(単一ピボットでの)目的関数改善量の推定に使う。
///
/// `lu_scratch`/`rho`/`a_p`/`touched`/`touched_cols` は呼び出し側の再利用バッファで、入口で
/// 0/空であることを前提とし、戻る前に同じ状態へ戻す。`d_dir` は行の逸脱方向、`d` は被約費用。
#[allow(clippy::too_many_arguments)]
fn trial_row_ratio(
    std: &StdForm,
    lu: &sparse_lu::FtLu,
    nb_status: &[Option<NbStatus>],
    d: &[f64],
    d_dir: i32,
    lu_scratch: &mut [f64],
    rho: &mut [f64],
    a_p: &mut [f64],
    touched: &mut [bool],
    touched_cols: &mut Vec<usize>,
    row: usize,
) -> Option<f64> {
    lu.solve_transpose_unit(row, lu_scratch, rho);

    for i in 0..std.n_rows {
        let rv = rho[i];
        if rv.abs() <= TOL {
            continue;
        }
        for &(j, v) in std.rows.row(i) {
            if std.lb[j] == std.ub[j] {
                continue;
            }
            if !touched[j] {
                touched[j] = true;
                touched_cols.push(j);
            }
            a_p[j] += rv * v;
        }
    }

    let mut best_ratio = f64::INFINITY;
    for &j in touched_cols.iter() {
        let Some(status) = nb_status[j] else { continue };
        let alpha_j = a_p[j];
        if alpha_j.abs() <= TOL {
            continue;
        }
        let sigma = nb_sigma(status, d_dir as f64, alpha_j);
        let hat_alpha = sigma * alpha_j;
        if (d_dir as f64) * hat_alpha >= 0.0 {
            continue;
        }
        // `Zero` 列の被約費用は不変条件により 0(論文の注意 3.1)なので比も 0。
        let hat_c = if status == NbStatus::Zero { 0.0 } else { (sigma * d[j]).max(0.0) };
        let ratio = hat_c / hat_alpha.abs();
        if ratio < best_ratio {
            best_ratio = ratio;
        }
    }

    for &j in touched_cols.iter() {
        a_p[j] = 0.0;
        touched[j] = false;
    }
    touched_cols.clear();

    best_ratio.is_finite().then_some(best_ratio)
}

/// `hat_u_j(M) - hat_l_j(M)`(列 `j` の幅)を `M` のアフィン関数として返す。幅が真の無限
/// (片側のみのスラック)なら `None`: その列はフリップできず、直接ピボットの対象にしかならない。
/// 自由列(両側 `M` 追跡)では `(0 + M) - (0 - M) = 2M`(`s_j = 2`)、片側列では `s_j = 1`。
fn width_affine(lower: Option<Affine1>, upper: Option<Affine1>) -> Option<Affine1> {
    match (lower, upper) {
        (Some(lo), Some(hi)) => Some(hi.sub(lo)),
        _ => None,
    }
}

/// 各列の [`hat_lower`]/[`hat_upper`]/[`width_affine`] のキャッシュ。主ループ開始前に一度だけ
/// 作る。どれも `std.lb`/`std.ub` と `n_orig` だけで決まり `nb_status` に依存しないので、
/// 求解中に無効化する必要が無い(chuzr/chuzc の内側ループでの再計算を避ける)。
struct ColCache {
    /// 各列の(`Lower` に置いたときの)下側の値。真の無限なら `None`。
    lower: Vec<Option<Affine1>>,
    /// 各列の(`Upper` に置いたときの)上側の値。真の無限なら `None`。
    upper: Vec<Option<Affine1>>,
    /// 各列の幅 `upper - lower`。どちらかが `None` なら `None`。
    width: Vec<Option<Affine1>>,
}

impl ColCache {
    /// 元問題(`M`-アフィン境界)のキャッシュを作る。
    fn build(std: &StdForm, n_orig: usize) -> Self {
        let n_total = std.n_total;
        let lower: Vec<Option<Affine1>> = (0..n_total).map(|j| hat_lower(std, n_orig, j)).collect();
        let upper: Vec<Option<Affine1>> = (0..n_total).map(|j| hat_upper(std, n_orig, j)).collect();
        let width: Vec<Option<Affine1>> = (0..n_total).map(|j| width_affine(lower[j], upper[j])).collect();
        ColCache { lower, upper, width }
    }

    /// 段階 A(傾き問題、論文の式 (7))の境界: 各側の `M` 係数だけを残す(`l^1 ∈ {-1, 0}`、
    /// `u^1 ∈ {0, 1}`、有限側は 0)。真の無限(不等式行のスラック)は無いまま。幅はちょうど
    /// 傾きチャネルの BFRT フリップ容量 `s_j` になる。
    fn slope_problem(orig: &ColCache) -> Self {
        let slope_only = |b: &Option<Affine1>| b.map(|a| Affine1::new(0.0, a.slope));
        let lower: Vec<Option<Affine1>> = orig.lower.iter().map(slope_only).collect();
        let upper: Vec<Option<Affine1>> = orig.upper.iter().map(slope_only).collect();
        let width = lower.iter().zip(&upper).map(|(&lo, &hi)| width_affine(lo, hi)).collect();
        ColCache { lower, upper, width }
    }

    /// 段階 B(切片問題、論文の式 (8))の境界を、段階 A の最適な傾きベクトル `x^1`(基底は
    /// `x_b_slope`、非基底は `nb_status` の側の傾き)から作る。`x^1_j` がその側の傾き境界上に
    /// ある(`SLOPE_TOL` 以内)ときだけその側が切片(`l_j`、人工 `-M` なら 0)として残り、
    /// そうでなければ `x_j` は `M` の正の倍数だけ離れているので捨てる(`±inf`)。段階 B の間
    /// `x^1` は変わらない(論文の命題 7.2 (ii))ので境界も固定。`orig` に無い側に非基底状態が
    /// あれば `None`(このモジュールのピボットでは起きない)。
    fn intercept_problem(orig: &ColCache, nb_status: &[Option<NbStatus>], basis: &[usize], x_b_slope: &[f64]) -> Option<Self> {
        let n_total = orig.lower.len();
        // 段階 A の最適な傾きベクトル `x^1`(変数番号で添字付け)。
        let mut x1 = vec![0.0f64; n_total];
        for (j, s) in nb_status.iter().enumerate() {
            x1[j] = match s {
                None | Some(NbStatus::Zero) => 0.0,
                Some(NbStatus::Lower) => orig.lower[j]?.slope,
                Some(NbStatus::Upper) => orig.upper[j]?.slope,
            };
        }
        for (pos, &j) in basis.iter().enumerate() {
            x1[j] = x_b_slope[pos];
        }
        let lower: Vec<Option<Affine1>> = (0..n_total)
            .map(|j| orig.lower[j].and_then(|lo| (x1[j] <= lo.slope + SLOPE_TOL).then_some(Affine1::new(lo.base, 0.0))))
            .collect();
        let upper: Vec<Option<Affine1>> = (0..n_total)
            .map(|j| orig.upper[j].and_then(|hi| (x1[j] >= hi.slope - SLOPE_TOL).then_some(Affine1::new(hi.base, 0.0))))
            .collect();
        let width = lower.iter().zip(&upper).map(|(&lo, &hi)| width_affine(lo, hi)).collect();
        Some(ColCache { lower, upper, width })
    }
}

/// 傾き・切片二段解法の本体(論文 6 節・7 節、Algorithm 1)。数値の `M` を固定せずに、`M` 切り詰め問題で
/// 主実行可能な状態(段階 A: 傾き問題 → 段階 B: 切片問題)に到達し、その後
/// [`finish`] で終了判定・cleanup・真の境界での仕上げを行う。
///
/// 引数: `std` は presolve 後の標準形 LP(連結成分 1 つ分)、`opts` は LP オプション
/// (`distinguish_infeasible_unbounded` が偽なら `z^1 < 0` の時点で
/// `InfeasibleOrUnbounded` を返す)。戻り値 `None` は「到達しないはず」の数値的破綻や
/// 反復上限到達で、呼び出し側が `Status::NotSolved` として報告する。
///
/// 再分解が特異基底で失敗して `None` になったときだけ、`safe_pivot` を有効にして最初から
/// 1 回だけ解き直す(`STUCK_ROW_MIN_PIVOT` 参照)。通常の求解が成功する問題では
/// 経路は変わらず、数値的に破綻した求解だけを救済する。
pub fn solve_slope_intercept_dual(std: &StdForm, opts: &crate::types::LpOptions) -> Option<SimplexResult> {
    SINGULAR_BAILOUT.with(|f| f.set(false));
    RESTART_BAILOUT.with(|f| f.set(false));
    let res = solve_slope_intercept_dual_with(std, opts, false);
    let singular = SINGULAR_BAILOUT.with(|f| f.get());
    let uncertified = RESTART_BAILOUT.with(|f| f.get());
    if res.is_some() || !(singular || uncertified) {
        return res;
    }
    if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
        eprintln!("DEBUG_EXT: {} bailout, retrying with safe_pivot and full cost perturbation", if uncertified { "broken-basis/max-iters" } else { "singular-basis" });
    }
    // 作業 #8 対処 5: 解き直しでは費用摂動を既定の大きさに戻す (試験用の縮小 `ENOMOTO_T_PERTURB_*FACTOR` を無視する。
    // 既定の設定では摂動は変わらない)。
    super::set_full_cost_perturbation(true);
    let res = solve_slope_intercept_dual_with(std, opts, true);
    super::set_full_cost_perturbation(false);
    res
}

/// [`solve_slope_intercept_dual`] の本体。`safe_pivot` が真なら、updateVerify で破棄された直後の行を
/// 再試行するとき、より大きなピボット候補があれば極小ピボットを避ける(`STUCK_ROW_MIN_PIVOT` 参照)。
fn solve_slope_intercept_dual_with(std: &StdForm, opts: &crate::types::LpOptions, safe_pivot: bool) -> Option<SimplexResult> {
    // 行数が `SPARSE_PATH_MIN_M` 以上なら新しい疎経路を含む実体 (`BIG = true`)、未満なら元の経路の
    // コードだけの実体を使う (小さな問題のホット経路のコード量・配置を変えないため。
    // `sparse_lu::sparse_path_min_m` 参照)。どちらも結果はビット一致 (`m >= 10,000` でゲートした
    // 経路変更は `BIG` の中にある)。
    if std.n_rows >= sparse_lu::sparse_path_min_m() {
        solve_slope_intercept_dual_impl::<true>(std, opts, safe_pivot)
    } else {
        solve_slope_intercept_dual_impl::<false>(std, opts, safe_pivot)
    }
}

/// 双対単体法の主ループと仕上げの段 ([`polish_with_true_bounds`]) が共有する摂動済み費用。
///
/// `super::perturb_costs` の出力のうち、スラック列 (`n_orig..n_total`) だけを真の (0 の) 費用に
/// 戻す: スラックを摂動すると全スラック基底でも `y = B^-T c_B` が非ゼロになり、`active_cost` の
/// 符号で置く `crash` が双対実行不能から始まってしまう。摂動しなければ開始時 `y = 0` で crash は
/// 構成上双対実行可能。polish も同じ費用を使わないと、主ループが最適と判断した基底が polish の
/// 費用では (スラックの摂動の分だけ) 双対実行不能になりうる。
fn dual_active_costs(std: &StdForm) -> Vec<f64> {
    let n_orig = std.n_total - std.n_rows;
    let mut active_cost = super::perturb_costs(std);
    for j in n_orig..std.n_total {
        active_cost[j] = std.c[j];
    }
    active_cost
}

/// [`solve_slope_intercept_dual_with`] の本体。`BIG` は新しい疎経路 (stormG2 報告 §4 の策) を使うか。
fn solve_slope_intercept_dual_impl<const BIG: bool>(std: &StdForm, opts: &crate::types::LpOptions, safe_pivot: bool) -> Option<SimplexResult> {
    // 全列数(構造列+スラック列)、行数、構造列数。スラック列は `n_orig..n_total`。
    let n_total = std.n_total;
    let m = std.n_rows;
    let n_orig = n_total - m;

    // コスト摂動([`dual_active_costs`])。`d` の増分維持と `crash` はこの摂動済みコストに基づく。
    let mut active_cost = dual_active_costs(std);

    // `delta[j]`: 列 `j` の `M` 追跡側([`delta_of`] 参照。現在は S 制限なしで、片側非有界・
    // 自由な構造列をすべてフラグする)。スラック列は常に `MSide::None`。
    let delta: Vec<MSide> = (0..n_total).map(|j| if j < n_orig { delta_of(std, j) } else { MSide::None }).collect();
    // `cache_orig`: 真の `M`-アフィン境界(`hat_l`, `hat_u`)。`finish` の cleanup と `m == 0`
    // の近道で使う。主ループ自体は現在の段階が解く問題の境界 `cache` で動く([`Phase`] 参照)。
    let cache_orig = ColCache::build(std, n_orig);
    // 現在の段階。既定は段階 A から始める(`ENOMOTO_LEX_EXTENDED=1` で旧来の単一ループ `Lex`)。
    let mut phase = if env_str!("ENOMOTO_LEX_EXTENDED").is_some_and(|v| v != "0") { Phase::Lex } else { Phase::A };

    if m == 0 {
        // 制約が無い: 各列を(摂動済み)コストの符号が好む側に置く。その側が真の `M` 側で
        // コストが非ゼロなら非有界。
        let mut x = vec![0.0; n_total];
        for j in 0..n_orig {
            let status = if active_cost[j] >= -TOL { NbStatus::Lower } else { NbStatus::Upper };
            let val = nb_value_affine(&cache_orig, status, j)?;
            if val.slope != 0.0 && active_cost[j].abs() > TOL {
                return Some(SimplexResult { status: Status::Unbounded, x: None });
            }
            x[j] = val.base;
        }
        return Some(SimplexResult { status: Status::Optimal, x: Some(x) });
    }

    // 基底(行位置 → 変数番号)。初期は全スラック基底。
    let mut basis: Vec<usize> = (n_orig..n_total).collect();
    // 変数番号 → 基底位置(非基底なら `None`)。
    let mut basis_pos: Vec<Option<usize>> = vec![None; n_total];
    for (col, &v) in basis.iter().enumerate() {
        basis_pos[v] = Some(col);
    }
    // 非基底変数の状態(`Lower`/`Upper`/`Zero`、基底変数は `None`)。コスト符号による crash で初期化。
    let mut nb_status = crash(std, &active_cost, n_orig);
    // 実験的機能(既定オフ、`refine_zero_cost_placement` 参照)。
    if env_str!("ENOMOTO_CRASH_ZERO_COST_PLACEMENT").is_some_and(|v| v != "0") {
        refine_zero_cost_placement(std, &mut active_cost, &mut nb_status, n_orig);
    }
    // 段階 A が空になる場合: crash がどの非基底列も `M` 側に置かなければ(`S = empty`)、
    // 全スラック基底で `x^1 = 0` がすでに傾き問題の最適なので、段階 A を飛ばして元の境界で
    // 段階 B から始める。`cache`: 現在の段階が解く問題の列境界。
    let mut cache = if phase == Phase::A
        && nb_status.iter().enumerate().all(|(j, s)| match s {
            Some(NbStatus::Lower) => cache_orig.lower[j].map_or(true, |a| a.slope == 0.0),
            Some(NbStatus::Upper) => cache_orig.upper[j].map_or(true, |a| a.slope == 0.0),
            Some(NbStatus::Zero) | None => true,
        })
    {
        phase = Phase::B;
        if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
            eprintln!("DEBUG_EXT: stage A skipped (S empty)");
        }
        ColCache::intercept_problem(&cache_orig, &nb_status, &basis, &vec![0.0; m])?
    } else {
        match phase {
            Phase::A => ColCache::slope_problem(&cache_orig),
            _ => ColCache::build(std, n_orig),
        }
    };
    // `cache.width[j].is_none()`(幅が真の無限)を 1 列 1 バイトで持つ。chuzc1 の停止候補判定用。
    let mut width_inf: Vec<bool> = cache.width.iter().map(|w| w.is_none()).collect();

    // この求解のピボット閾値の段階的引き上げを既定値から始める(スレッドローカルなので
    // 前の求解が上げた閾値を引き継がないようリセットする)。
    sparse_lu::reset_pivot_threshold();
    // 現基底の LU 分解(Forrest-Tomlin 更新付き)。
    let mut lu = refactorize(std, &basis_pos, None)?;
    // 前回の FT チェック(`XB_CHECK_INTERVAL` 周期)からの反復数。
    let mut since_check = 0usize;

    // 被約費用 `d`(Huangfu & Hall §2.2.3 の update-dual で増分維持)。列のコストは `M` に
    // 依存しない(依存するのは境界だけ)ので `d` は通常の `f64` 配列。全スラック基底では
    // `y = 0` なので `d = active_cost` で正しい。
    let mut d = active_cost.clone();
    // `fresh_d_into` 用の作業領域(`c_B` と `y = B^-T c_B`)。
    let mut fresh_d_cb = vec![0.0f64; m];
    let mut fresh_d_y = vec![0.0f64; m];
    // `d` のドリフト検査([`D_DRIFT_TOL`])用に作り直した `d`(比較後に捨てる)。
    let mut fresh_d_buf = vec![0.0f64; n_total];

    // ===== 作業バッファ(ループ前に一度だけ確保し、毎反復再利用する) =====
    // `a_p`: ピボット行 `rho^T A`(列添字)。`touched`/`touched_cols`: `a_p` の非ゼロ列の印と一覧。
    let mut a_p = vec![0.0f64; n_total];
    let mut touched = vec![false; n_total];
    let mut touched_cols: Vec<usize> = Vec::new();
    // `x_B(M) = x_b_base + x_b_slope * M`(基底位置順)。
    let mut x_b_base = vec![0.0f64; m];
    let mut x_b_slope = vec![0.0f64; m];
    // LU 求解(密)用の作業領域。
    let mut lu_scratch = vec![0.0f64; m];
    // [`residual_norm_affine`] のドリフト検査用作業領域。
    let mut resid_scratch_base = vec![0.0f64; m];
    let mut resid_scratch_slope = vec![0.0f64; m];
    // `rho = B^-T e_r`(ピボット行の BTRAN 結果)。
    let mut rho = vec![0.0f64; m];
    // 疎 PRICE(S8)用の `rho` の非ゼロ行一覧(昇順)と、密度予測に使う前反復の PRICE 行数。
    let mut rho_rows = vec![0u32; m];
    let mut last_rho_nnz = 0usize;
    // square41 報告の策3: 前反復の PRICE 要素数 (密結果モードの判定用)。
    let mut last_price_entries = 0usize;
    // 密結果モードに入る PRICE 要素数の閾値 (`n_total` に対する比、`ENOMOTO_T_PRICE_DENSE_RESULT`、0 = 無効)。
    let price_dense_ratio = tunable!("ENOMOTO_T_PRICE_DENSE_RESULT", PRICE_DENSE_RESULT_RATIO, f64);
    // 列添字でランダムに引く PRICE・chuzc1・`d` 更新のループでソフトウェアプリフェッチを使うか
    // (`BIG` かつ `n_total >= PREFETCH_MIN_COLS`。`a_p`・`d` がキャッシュに収まらない列数の多い問題用。
    // 値は変わらない。`ENOMOTO_T_PREFETCH_MIN_COLS` で閾値を上書き、0 = 無効)。
    let prefetch_on = BIG && {
        let min_cols = tunable!("ENOMOTO_T_PREFETCH_MIN_COLS", PREFETCH_MIN_COLS, usize);
        min_cols > 0 && n_total >= min_cols
    };
    // 先読み距離 (要素数、`ENOMOTO_T_PREFETCH_DIST`)。
    let prefetch_dist = tunable!("ENOMOTO_T_PREFETCH_DIST", PREFETCH_DIST, usize);
    // 入る列 `A_q` の密ベクトル(密 FTRAN の右辺。反復間は常に 0)。
    let mut dense_q = vec![0.0f64; m];
    // `alpha_full = B^-1 A_q`(入る列の FTRAN 結果)。
    let mut alpha_full = vec![0.0f64; m];
    // `alpha_full`(とフリップ結果)の非ゼロ行一覧(昇順)。`x_B` の疎更新用。
    let mut xb_rows = vec![0u32; m];
    // DSE の `tau = B^-1 rho`。
    let mut tau = vec![0.0f64; m];
    // 入る列/`tau` 融合 FTRAN の `tau` 側作業領域(入る列側は `lu_scratch`)。
    let mut tau_scratch = vec![0.0f64; m];
    // この反復の `rho` の BTRAN が記録した非ゼロステップ。融合 `tau` FTRAN の `L` 段を
    // Gilbert-Peierls 経路にするために使う([`sparse_lu::StepCapture`]、密 `L` 段とビット同一)。
    let mut rho_steps = sparse_lu::StepCapture::new(m);
    // 融合 FTRAN の出力(`alpha_full`/`tau`/`a_tilde_buf`)の非ゼロ位置の記録([`sparse_lu::FtranTrack`])。
    // 超疎経路では前回の位置だけを消して書くので、長さ `m` の `fill`/`copy` を省ける(ビット同一)。
    // `ENOMOTO_SPARSE_FTRAN_OUT=0` で従来の全体書き出し(A/B 用)。
    let sparse_ftran_out = BIG && env_str!("ENOMOTO_SPARSE_FTRAN_OUT").map_or(true, |v| v != "0");
    let mut ftran_track = sparse_lu::FtranTrack::new();
    // 作業 #10 (B): 部分 `tau` の反復は `tau` の密度の移動平均に入れない (`ENOMOTO_T_PARTIAL_TAU_SKIP_DENSITY=1`、
    // 既定 0 = 旧版。合成クロックの密度区分が変わり irish-electricity (旧前処理) の経路が特異基底に入ったので既定にしない)。
    let partial_tau_skip_density = tunable!("ENOMOTO_T_PARTIAL_TAU_SKIP_DENSITY", 0u8, u8) != 0;
    // 策12: 大きな問題では DSE の `tau` を入る列の結果の非ゼロ行でだけ求める(`PARTIAL_TAU_MIN_M`)。
    let partial_tau = sparse_ftran_out && {
        let min_m = tunable!("ENOMOTO_T_PARTIAL_TAU_MIN_M", PARTIAL_TAU_MIN_M, usize);
        min_m > 0 && m >= min_m
    };
    // ピボット行 BTRAN 専用の 0 維持作業領域と触れた位置の一覧([`sparse_lu::UnitBtranWork`])。
    let mut btran_work = sparse_lu::UnitBtranWork::new(m);
    // DSE の `tau` FTRAN を入る列の FTRAN と融合するか(`ENOMOTO_FUSED_DSE_FTRAN=0` で別々。ビット同一)。
    let fused_dse_ftran = env_str!("ENOMOTO_FUSED_DSE_FTRAN").map_or(true, |v| v != "0");
    // BFRT 結合フリップの(密分岐の)FTRAN を第 3 のベクトルとして同じ走査に融合するか
    // (`ENOMOTO_FUSED_BFRT_FTRAN=0` で別々。ビット同一)。`combined_scratch` はその作業領域。
    let fused_bfrt_ftran = env_str!("ENOMOTO_FUSED_BFRT_FTRAN").map_or(true, |v| v != "0");
    let mut combined_scratch = vec![0.0f64; m];
    // BFRT 結合フリップの結果を入る列の `x_B` 更新ループ内で反映するか(1 パス・1 行 1 回の
    // `refresh_row`)。`ENOMOTO_MERGE_FLIP_XB=0` で別パス。行ごとの演算は同じで、
    // `InfeasibleRows` のメンバーシップ変更の順序だけが変わりうる。
    let merge_flip_xb = env_str!("ENOMOTO_MERGE_FLIP_XB").map_or(true, |v| v != "0");
    // `try_update_precomputed` 用のキャプチャバッファ: `e_tilde_buf` は `rho` の BTRAN、
    // `a_tilde_buf` は入る列の FTRAN の副産物として埋まる。キャプチャから使用までの間に
    // 他の何もこれらに書き込んではならない。
    let mut a_tilde_buf = vec![0.0f64; m];
    let mut e_tilde_buf = vec![0.0f64; m];
    // 比率テストの候補(chuzc1 の出力)。
    let mut candidates: Vec<Cand> = Vec::new();
    // chuzc1 のヒープ用領域(反復をまたいで再利用)。
    let mut heap_buf: Vec<Reverse<Cand>> = Vec::new();
    // chuzc1 の分岐なしフィルタ用の書き込み領域と、非 bland 歩進でヒープから取り出した先頭部分。
    let mut cand_scratch: Vec<Cand> = Vec::new();
    let mut sorted_prefix: Vec<Cand> = Vec::new();
    // 策8: chuzc1 の上位候補の選択数 (`BIG` のみ、0 = 全体ヒープ。`ENOMOTO_T_CHUZC1_TOPK`)、
    // 上限付き最大ヒープと並べた結果。
    let chuzc1_topk: usize = if BIG { tunable!("ENOMOTO_T_CHUZC1_TOPK", CHUZC1_TOPK, usize) } else { 0 };
    // 上位候補の選択を使う最小候補数 (`ENOMOTO_T_CHUZC1_TOPK_MIN_CANDS`、これ未満は全体ヒープのほうが安い)。
    let chuzc1_topk_min_cands: usize = tunable!("ENOMOTO_T_CHUZC1_TOPK_MIN_CANDS", CHUZC1_TOPK_MIN_CANDS, usize).max(chuzc1_topk + 1);
    // 策7 (停止候補の刈り込みの省略): 刈り込みで候補がほとんど減らない問題 (s250r10) では刈り込みの走査
    // (`width_inf` のランダムな読み) と候補の写しが無駄だが、片側行のスラックや上限のない列が多い問題
    // (Netlib の多く) では刈り込みで候補が桁違いに減り、省くと遅くなる (scsd8・pilot.we などで 10〜19%)。
    // 上位候補の選択を使う反復の `CHUZC1_FAST_PROBE` 回に 1 回は刈り込みを行って残った割合を測り、半分より多く
    // 残ったら次の測定まで省く。どちらでも結果はビット一致。`ENOMOTO_T_CHUZC1_FAST=0` で常に刈り込む。
    // 列数の多い問題 (`prefetch_on` と同じ `n_total >= PREFETCH_MIN_COLS`) だけ: 小さな問題では `width_inf` が
    // キャッシュに乗っていて刈り込みが安い。
    let chuzc1_fast_allowed = prefetch_on && tunable!("ENOMOTO_T_CHUZC1_FAST", 1u8, u8) != 0;
    let mut chuzc1_fast_on = false;
    let mut chuzc1_fast_probe: usize = 0;
    let mut topk_heap: BinaryHeap<Cand> = BinaryHeap::with_capacity(chuzc1_topk);
    let mut topk_sorted: Vec<Cand> = Vec::with_capacity(chuzc1_topk);

    // 経路を変える・既定オン(`ENOMOTO_PRICE_NONBASIC_ONLY=0` で全非固定列を PRICE):
    // HiGHS の行方向分割 PRICE 行列と同じく、各行の非基底要素を `[start, p_end)` に、
    // 基底要素をその後ろに置き、基底交換のたびに境界をまたいで入れ替えるので、PRICE は
    // 基底列を一切訪れない。離基列の `d` は HiGHS `HEkkDual::updateDual` と同様に直接設定する
    // (無限精度では同値だがビット同一ではない)。`price_nb_end[i]` は行 `i` の分割境界
    // (フラグオフなら単に行末)。
    let price_nonbasic_only = env_str!("ENOMOTO_PRICE_NONBASIC_ONLY").map_or(true, |v| v != "0");
    // S12: 分割の入れ替え用の位置索引。`col_entry_start[j]` は列 `j` の `std.cols` 要素順での開始位置、
    // `price_pos_of_col_entry[col_entry_start[j] + k]` は `std.cols.col(j)[k]` の PRICE 行列内の位置(固定列は
    // `u32::MAX`)、`col_entry_of_price[p]` はその逆写像(PRICE 要素 `p` の `std.cols` 要素番号)。
    let mut col_entry_start: Vec<usize> = Vec::with_capacity(if price_nonbasic_only { n_total + 1 } else { 0 });
    let mut price_pos_of_col_entry: Vec<u32> = Vec::new();
    let mut col_entry_of_price: Vec<u32> = Vec::new();
    // PRICE 専用の `A` の行優先コピー(一度だけ構築): `std.rows` から固定列 (`lb == ub`) を除き、
    // 列添字 `u32` の SoA 形式(`price_start`/`price_col`/`price_val`)。各行は `std.rows.row(i)`
    // の列順を(分割の各部分内で)保つので `a_p` の各ビットは不変。`price_nonbasic_only` の
    // ときは各行を「非基底部 → 基底部」の順で書く。
    let mut price_nb_end: Vec<usize> = Vec::with_capacity(m);
    let (price_start, mut price_col, mut price_val) = {
        let mut start: Vec<usize> = Vec::with_capacity(m + 1);
        let mut col: Vec<u32> = Vec::with_capacity(std.rows.nnz());
        let mut val: Vec<f64> = Vec::with_capacity(std.rows.nnz());
        // 各 PRICE 要素の `std.cols` 要素番号を列ごとのカーソルで求める(行は昇順に訪れるので、
        // 行ソート済みの列ではカーソルが常に一致位置にある)。
        let mut cursor: Vec<u32> = Vec::new();
        if price_nonbasic_only {
            col_entry_start.push(0);
            for j in 0..n_total {
                col_entry_start.push(col_entry_start[j] + std.cols.col(j).len());
            }
            cursor = vec![0; n_total];
            col_entry_of_price.reserve(std.rows.nnz());
        }
        start.push(0);
        for i in 0..m {
            if price_nonbasic_only {
                for want_nonbasic in [true, false] {
                    for &(j, v) in std.rows.row(i) {
                        if std.lb[j] == std.ub[j] || nb_status[j].is_some() != want_nonbasic {
                            continue;
                        }
                        col.push(u32::try_from(j).ok()?);
                        val.push(v);
                        let c = std.cols.col(j);
                        let cur = cursor[j] as usize;
                        let k = if cur < c.len() && c[cur].0 == i { cur } else { c.iter().position(|&(r, _)| r == i)? };
                        cursor[j] = (k + 1) as u32;
                        col_entry_of_price.push(u32::try_from(col_entry_start[j] + k).ok()?);
                    }
                    if want_nonbasic {
                        price_nb_end.push(col.len());
                    }
                }
            } else {
                for &(j, v) in std.rows.row(i) {
                    if std.lb[j] == std.ub[j] {
                        continue;
                    }
                    col.push(u32::try_from(j).ok()?);
                    val.push(v);
                }
                price_nb_end.push(col.len());
            }
            start.push(col.len());
        }
        (start, col, val)
    };
    if price_nonbasic_only {
        price_pos_of_col_entry = vec![u32::MAX; col_entry_start[n_total]];
        for (p, &e) in col_entry_of_price.iter().enumerate() {
            price_pos_of_col_entry[e as usize] = p as u32;
        }
    }
    // S9(`ENOMOTO_PRICE_COLUMN=1`、既定オフ): `rho` が密(`ENOMOTO_PRICE_COLUMN_DENSITY`、
    // 既定 0.1 = HiGHS の切り替え点)なら列方向 PRICE を使う。`price_col_list` は非固定列の一覧。
    let price_by_column = tunable!("ENOMOTO_PRICE_COLUMN", 0u8, u8) != 0;
    let price_column_density = tunable!("ENOMOTO_PRICE_COLUMN_DENSITY", PRICE_COLUMN_DENSITY, f64);
    let price_col_list: Vec<u32> = if price_by_column { (0..std.n_total).filter(|&j| std.lb[j] != std.ub[j]).map(|j| j as u32).collect() } else { Vec::new() };

    // `solve_sparse_into` 専用の作業領域(前提条件により `lu_scratch` とは共有しない)と
    // Gilbert-Peierls 用の作業領域。
    let mut sparse_scratch = vec![0.0f64; m];
    let mut gp_scratch = sparse_lu::GpScratch::new(m);

    // 呼び出し箇所ごとの FTRAN **結果**密度の移動平均。入力の非ゼロ数と合わせて密/疎ソルブを
    // 切り替える(入力が疎でも `L` のフィルインで結果が密になりうるため)。入る列の FTRAN と
    // BFRT 結合フリップの FTRAN は右辺の性質が違うので別々に持つ。求解全体の性質なので
    // 再分解をまたいで保持する(HiGHS も `HEkk` で同様)。
    let mut density_col_aq = sparse_lu::FtranDensity::new();
    let mut density_bfrt = sparse_lu::FtranDensity::new();
    // C5: 融合 DSE `tau` FTRAN の結果密度(その `U` 段を超疎にするかの判定用)。
    let mut density_tau = sparse_lu::FtranDensity::new();

    // BFRT 結合フリップの累積右辺(`Affine1` の 2 チャネル): この反復でフリップする全候補の
    // 寄与の和(`combined_base`/`combined_slope`)と、その非ゼロ行の印と一覧。
    let mut combined_base = vec![0.0f64; m];
    let mut combined_slope = vec![0.0f64; m];
    let mut combined_touched_flag = vec![false; m];
    let mut combined_touched: Vec<usize> = Vec::new();
    // 疎ソルブ分岐の入力バッファ(毎反復再利用)。
    let mut sparse_base_buf: Vec<(usize, f64)> = Vec::with_capacity(m);
    let mut sparse_slope_buf: Vec<(usize, f64)> = Vec::with_capacity(m);
    // 結合フリップの FTRAN 結果(`B^-1 × 累積右辺`、チャネルごと)。
    let mut combined_alpha_base = vec![0.0f64; m];
    let mut combined_alpha_slope = vec![0.0f64; m];
    // pds-100 報告の策13 の一部 (`BIG` のみ): 合成フリップ列の FTRAN 結果 (`combined_alpha_base`/`_slope`) の
    // 非ゼロ位置の記録。超疎に解けた反復は長さ `m` の `fill` を省き、`x_B` 更新の行一覧もこの記録から作る
    // (`compact_rows` の `O(m)` 走査を省く)。記録の外で全体を書いたら `set_full` で無効化する。
    let mut cab_track = sparse_lu::NzTrack::new();
    let mut cas_track = sparse_lu::NzTrack::new();
    // 記録を使うか (`m >= FLIP_TRACK_MIN_M`、`ENOMOTO_T_FLIP_TRACK_MIN_M`、0 = 無効): 小さな問題では `fill` も
    // `compact_rows` も安く、一覧の和集合の並べ替えのほうが高い。
    let flip_track = BIG && {
        let v = tunable!("ENOMOTO_T_FLIP_TRACK_MIN_M", FLIP_TRACK_MIN_M, usize);
        v > 0 && m >= v
    };
    // 作業 #10 (M): BFRT 結合フリップ列の FTRAN の出力は、`m` によらず (`BIG` なら) 前回書いた位置だけを消す記録版で書く
    // (記録なし版は毎回 `O(m)` の `fill`、ken-11 で全命令の 11.5%)。値・tick は同じ。行一覧の和集合で `x_B` を更新する
    // 経路 (`flip_track`、下の `union_lists`) は従来どおり `m >= FLIP_TRACK_MIN_M` だけ (小さな問題では和集合のソートが高い)。
    // `ENOMOTO_T_FLIP_SOLVE_TRACK=0` で旧版。
    let flip_solve_track = flip_track || (BIG && tunable!("ENOMOTO_T_FLIP_SOLVE_TRACK", 1u8, u8) != 0);
    // 行一覧の和集合を作る作業領域。
    let mut xb_union: Vec<usize> = Vec::new();

    // `x_B(M)` の初期値(論文の補題 6.1 の `b - N x_N` を一度だけ解く)。以後は増分で維持し、
    // 周期的な再同期で同じ計算に合わせ直す。
    let (seed_base, seed_slope) = resync_x_b(std, &cache, &nb_status, &lu, phase, &mut lu_scratch, &mut x_b_base, &mut x_b_slope)?;
    // `b - N x_N(M)` を増分維持するか(BFRT フリップと基底交換が自列の寄与を反映する)。
    // ドリフト検査のたびに `O(nnz(A))` で再計算しないため。完全な再同期のたびに
    // `compute_rhs_affine` の値に合わせ直す。`ENOMOTO_XB_RHS_INCREMENTAL=0` で毎回再計算。
    let rhs_incremental = env_str!("ENOMOTO_XB_RHS_INCREMENTAL").map_or(true, |v| v != "0");
    // 増分維持している右辺 `b - N x_N(M)` の基底・傾き成分(空の傾きは `delta = 0` を表す)。
    let mut rhs_inc_base = seed_base;
    let mut rhs_inc_slope = seed_slope;

    // `noise_feasible[j]`: Eligible が空になった時点で、逸脱がその行の丸め誤差程度
    // (行の RHS の大きさで尺度付け)と判定された基底変数の印。以後の chuzr で除外する。
    // 行位置は変わりうるので変数番号で添字付けする。
    let mut noise_feasible = vec![false; n_total];

    // 作業 #5 (特異基底への耐性) M1/M1'/M2 の重みに応じた雑音判定([`NOISE_C`] 参照)。
    // `noise_c`: 係数 `C`(0 で無効)。`noise_min_w`: 判定を適用する DSE 重みの下限(`NOISE_MIN_SQRT_W^2`)。
    // `noise_row_k2`: M1 の行の判定 `dev^2 <= noise_row_k2 * w_r`(`= (C eps max(‖b‖∞, 1))^2`)。
    // `infeas_guard_w`: M2 の `sqrt(w_r)` の閾値の 2 乗。`noise_dry`: 判定だけ数えて何もしない(計測用)。
    let noise_c: f64 = tunable!("ENOMOTO_T_NOISE_C", NOISE_C, f64);
    let noise_on = noise_c > 0.0;
    let noise_min_w: f64 = {
        let s = tunable!("ENOMOTO_T_NOISE_MIN_SQRT_W", NOISE_MIN_SQRT_W, f64);
        s * s
    };
    let noise_row_k2: f64 = if noise_on {
        let b_scale = std.b.iter().fold(0.0f64, |a, &v| a.max(v.abs())).max(1.0);
        let k = noise_c * f64::EPSILON * b_scale;
        k * k
    } else {
        0.0
    };
    let infeas_guard_w: f64 = {
        let s = tunable!("ENOMOTO_T_INFEAS_GUARD_SQRT_W", INFEAS_GUARD_SQRT_W, f64);
        s * s
    };
    let noise_dry = tunable!("ENOMOTO_T_NOISE_DRY", 0u8, u8) != 0;
    // 作業 #8 対処 2: M2 の Farkas 証明 (`infeas_check!`) を雑音判定 (`noise_on`) と独立に常に行う
    // (`ENOMOTO_T_INFEAS_CERTIFY=0` で作業 #5 の動作 = `noise_on` のときだけ)。
    let infeas_certify = tunable!("ENOMOTO_T_INFEAS_CERTIFY", 1u8, u8) != 0;
    // 作業 #8 対処 5: 証明できない実行不能の結論で (閾値を上げきったら) 解き直すか (`ENOMOTO_T_UNCERTIFIED_RESTART=0` で作業 #5 の動作)。
    let uncertified_restart = tunable!("ENOMOTO_T_UNCERTIFIED_RESTART", 1u8, u8) != 0;
    // 作業 #8 対処 3 (費用シフト、[`DEGEN_SHIFT_RUN`]): 連続した厳密な退化ピボットの数、シフトする連続回数 (0 = 無効)、
    // シフトの基準の大きさ (摂動と同じ `super::cost_perturb_base`)。診断: 退化ピボットの総数・最長の連続・シフトの回数と列数。
    let mut degen_run = 0usize;
    let degen_shift_run: usize = tunable!("ENOMOTO_T_DEGEN_SHIFT_RUN", DEGEN_SHIFT_RUN, usize);
    let degen_shift_base = if degen_shift_run > 0 { super::cost_perturb_base(std) } else { 0.0 };
    let degen_dj_tol: f64 = tunable!("ENOMOTO_T_DEGEN_DJ_TOL", DEGEN_DJ_TOL, f64);
    let mut diag_degen = 0usize;
    let mut diag_degen_max_run = 0usize;
    let mut diag_shift_events = 0usize;
    let mut diag_shift_cols = 0usize;
    let xb_consistency_tol: f64 = tunable!("ENOMOTO_T_XB_CONSISTENCY_TOL", XB_CONSISTENCY_TOL, f64);
    let mut diag_xb_mismatch = (0.0f64, 0usize);
    // M1' 用の列の無限大ノルム `‖a_j‖∞`(初めて必要になったときに作る)。
    let mut col_inf_norm: Vec<f64> = Vec::new();
    // 診断(`ENOMOTO_DEBUG_EXT_ITERS`): 選んだ行の DSE 重みの最大、M1 で外した行数、M1' で候補を外した
    // 反復数・候補数・全滅でタブーにした回数、M2 のガード回数、M3 の巻き戻し回数。
    let noise_diag = env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some();
    let mut diag_max_pick_w = 0.0f64;
    let mut diag_noise_rows = 0usize;
    let mut diag_noise_pivot_iters = 0usize;
    let mut diag_noise_pivot_cands = 0usize;
    let mut diag_noise_taboo = 0usize;
    let mut diag_infeas_guard = 0usize;
    let mut diag_rollbacks = 0usize;
    // 作業 #5 M3: 直近の成功した再分解以降に確定したピボットと BFRT フリップの記録([`rollback_pivots`])。
    let pivot_rollback = tunable!("ENOMOTO_T_PIVOT_ROLLBACK", 1u8, u8) != 0;
    let mut pivot_log: Vec<PivotRec> = Vec::new();
    let mut pivot_flips: Vec<usize> = Vec::new();
    // 巻き戻したピボットの禁止: 禁止を持つ行の印と `(r, q)` の一覧。
    let mut rb_ban_row = vec![false; m];
    let mut rb_bans: Vec<(usize, usize)> = Vec::new();

    // 超疎 chuzr(`super::InfeasibleRows`): 主実行不能な基底行の集合を、`x_B` を書き換える
    // 各ループで増分維持する(毎反復の全走査をしない)。
    let mut infeasible_rows = InfeasibleRows::new(m);
    // プール内の各行の逸脱のキャッシュ([`RowDevCache`])。
    let mut row_dev = RowDevCache::new(m);
    // 基底行ごとの境界のキャッシュ([`RowBounds`])。
    let mut row_bounds = RowBounds::new(&cache, &basis, &noise_feasible);
    rebuild_rows(&mut infeasible_rows, &mut row_dev, m, &row_bounds, &x_b_base, &x_b_slope);
    // S14: 全スラック開始では基底コストがすべて `+0.0` なので `y = B^-T c_B` も正確に `+0.0` で、
    // `fresh_d_into` は `d = active_cost` を再現する。[`fresh_d_into_zero_y`] はそれを直接書き、
    // BTRAN の合成クロック tick だけを再現する(CLOCK トリガとピボット経路はビット単位で不変)。
    if fresh_d_into_zero_y(std, &lu, &basis, &basis_pos, &active_cost, &mut fresh_d_cb, &mut lu_scratch, &mut fresh_d_y, &mut d) {
        if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
            eprintln!("DEBUG_EXT: initial fresh_d skipped (c_B = 0)");
        }
    } else {
        fresh_d_into(std, &lu, &basis, &basis_pos, &active_cost, &mut fresh_d_cb, &mut lu_scratch, &mut fresh_d_y, &mut d);
    }

    // 離基行の重み(論文 5.4 節 Step 2(b) の `γ_i`、注意 7.7): `super::DseState` をそのまま使う(重みは `M` に依存しない
    // 表の行の量。`M` に依存するのはそれを使うスコア `Score2` だけ)。最初のピボットから
    // 厳密 DSE を使う。全スラックの `B0` は符号付き単位行列なので `DseState::new` の単位重みは正確。
    let mut dse = super::DseState::new(m);

    // 停滞(目的関数への寄与がほぼ 0 のピボット)がこの回数を超えたら Bland 規則に切り替える。
    let stall_limit = (STALL_LIMIT_PER_ROW * m).max(STALL_LIMIT_MIN);
    // 連続した停滞ピボットの数。
    let mut stall_count = 0usize;
    // Bland 規則(最小添字優先)による巡回防止モードか(一度入ると段階移行まで抜けない)。
    let mut bland_mode = false;
    // 実験的な「最大改善」chuzr エスカレーション: `stall_count` がこの閾値を超えたら、DSE の
    // 幾何的な代理指標ではなく、DSE 上位 `GREATEST_IMPROVEMENT_TOP_K` 行それぞれについて
    // 試行 BTRAN+PRICE+比率テスト([`trial_row_ratio`])で実際の双対目的関数の改善量を推定し、
    // 最大の行を選ぶ(`ENOMOTO_DISABLE_GREATEST_IMPROVEMENT` で無効)。
    let greatest_improvement_stall_threshold = (stall_limit / GREATEST_IMPROVEMENT_STALL_DIVISOR).max(GREATEST_IMPROVEMENT_STALL_MIN);
    let greatest_improvement_enabled = env_str!("ENOMOTO_DISABLE_GREATEST_IMPROVEMENT").is_none();
    let debug_greatest_improvement = env_str!("ENOMOTO_DEBUG_EXT_GREATEST_IMPROVEMENT").is_some();
    // 候補行 `(行, 方向, 逸脱, DSE スコア)` の作業領域。
    let mut greatest_improvement_cands: Vec<(usize, i32, Affine1, Score2)> = Vec::with_capacity(GREATEST_IMPROVEMENT_TOP_K + 1);
    // 反復上限(`super::max_iters_for`)。
    let max_iters = super::max_iters_for(m, n_total);
    // 実行不能行数プラトー検出(ループ本体の該当箇所参照)の上限反復数。健全だが遅い求解の
    // 通常の揺らぎを避けるため `stall_limit` より大きくするが、反復上限の予算内で発火できる
    // よう実際の反復予算 `max_iters / 4` で頭打ちにする(m + n ≤ 1,000 なら予算は
    // `MAX_ITERS_FLOOR` なので従来と同じ 5,000)。以前は予算によらず `MAX_ITERS_FLOOR / 4`
    // (= 5,000) 固定で、大きな問題 (pds-100, s250r10) では実行不能行数が振動するだけの健全な
    // 求解で誤発火して Bland 規則に落ち、収束しなくなっていた。
    // `ENOMOTO_T_PLATEAU_BUDGET_FLOOR=1` で従来の上限 (A/B 用)。
    let plateau_budget = if tunable!("ENOMOTO_T_PLATEAU_BUDGET_FLOOR", 0usize, usize) != 0 { MAX_ITERS_FLOOR } else { max_iters };
    let infeasible_plateau_limit = (INFEASIBLE_PLATEAU_STALL_MULT * stall_limit).min(plateau_budget / INFEASIBLE_PLATEAU_BUDGET_DIVISOR);
    // 最小の実行不能行数を更新できていない連続反復数。
    let mut infeasible_plateau_count = 0usize;
    // 双対目的関数の進展の尺度 (ループ本体のプラトー検出参照): 求解開始からの `|contribution_base|` の累積、
    // 最後にプラトー計数をリセットしてからの累積、進展とみなす相対量 (`ENOMOTO_T_PLATEAU_OBJ_REL`、0 = 無効)。
    let mut plateau_obj_total = 0.0f64;
    let mut plateau_obj_acc = 0.0f64;
    let plateau_obj_rel: f64 = tunable!("ENOMOTO_T_PLATEAU_OBJ_REL", PLATEAU_OBJ_REL, f64);
    // これまでに見た最小の実行不能行数(前反復の値ではない)。
    let mut best_infeasible_len = infeasible_rows.rows.len();
    // [`XB_DRIFT_TOL`](エスカレーションの出発点)の上書き(`ENOMOTO_XB_DRIFT_TOL`、A/B 用)。
    let xb_drift_tol: f64 = env_str!("ENOMOTO_XB_DRIFT_TOL").and_then(|s| s.parse::<f64>().ok()).unwrap_or(XB_DRIFT_TOL);
    // 丸め誤差の尺度に対する相対許容誤差([`XB_DRIFT_REL_TOL`]、0 = 無効)。
    let xb_drift_rel_tol: f64 = tunable!("ENOMOTO_XB_DRIFT_REL_TOL", XB_DRIFT_REL_TOL, f64);
    // この求解内でのドリフト起因の再分解回数([`XB_DRIFT_TOL`] の段階的緩和に使う)。
    let mut drift_trigger_count: usize = 0;
    // 再分解後最初のドリフト検査で測った残差(その分解自体の雑音水準)。
    let mut drift_resid_after_refactor: f64 = 0.0;
    // S2(`ENOMOTO_XB_DRIFT_SAMPLE`、既定オフ): ドリフト残差の巡回行サンプリングによる事前検査
    // ([`sampled_residual_affine`])。`k`(0 でオフ)、ガード係数、次に使う行オフセット。
    let xb_drift_sample: usize = tunable!("ENOMOTO_XB_DRIFT_SAMPLE", XB_DRIFT_SAMPLE_K, usize);
    let xb_drift_sample_guard: f64 = tunable!("ENOMOTO_XB_DRIFT_SAMPLE_GUARD", XB_DRIFT_SAMPLE_GUARD, f64);
    let mut drift_sample_offset: usize = 0;
    // 作業 #10 (C): 標本検査 (S2) で全体の検査を省いてよい連続回数 (`XB_CHECK_FULL_EVERY - 1`)。
    // `XB_CHECK_FULL_EVERY` 回に 1 回は標本によらず全体の検査を行う (標本に入らない行に偏ったドリフトの
    // 見逃しを、旧来の検査間隔の `XB_CHECK_FULL_EVERY` 倍以内に抑える)。0 = 上限なし (旧 S2)。
    let xb_check_full_every: usize = tunable!("ENOMOTO_T_XB_CHECK_FULL_EVERY", XB_CHECK_FULL_EVERY, usize);
    let mut drift_checks_since_full: usize = 0;
    // 作業 #10 (C): 直近に測った丸め誤差の尺度 (`residual_scale_affine`、基底・傾き)。未計測なら 0 (絶対許容誤差だけ)。
    let mut drift_last_scale: (f64, f64) = (0.0, 0.0);
    // 新規残差の下限(`ENOMOTO_XB_DRIFT_FRESH_FLOOR`、既定オフ): 再分解直後の残差がすでに
    // 許容誤差の `frac` 倍を超えているなら(再分解しても下がらない)、次の再分解まで
    // `factor * 新規残差` を許容誤差の下限とする。`xb_fresh_floor` は現在の下限(0 なら無効)。
    let xb_fresh_floor_factor: f64 = tunable!("ENOMOTO_XB_DRIFT_FRESH_FLOOR", XB_DRIFT_FRESH_FLOOR_FACTOR, f64);
    let xb_fresh_floor_frac: f64 = tunable!("ENOMOTO_XB_DRIFT_FRESH_FLOOR_FRAC", XB_DRIFT_FRESH_FLOOR_FRAC, f64);
    let mut xb_fresh_floor: f64 = 0.0;
    // `docs/lu_comparison_enomoto_vs_highs.md` §2.4 のピボット閾値の段階的引き上げ: 数値的原因による再分解
    // ([`PIVOT_ESCALATION_STEP`] 参照)を数え、その回数ごとに LU の閾値を上げる(0 で無効)。
    let pivot_escalation_step: usize = env_str!("ENOMOTO_PIVOT_ESCALATION_STEP")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(PIVOT_ESCALATION_STEP);
    // 数値的原因による再分解の回数。
    let mut numeric_trouble_count: usize = 0;
    // 作業 #5 M5 ([`PIVOT_ESCALATE_BELOW_DEFAULT`]・[`PIVOT_ESCALATE_FEW_UPDATES`]、HiGHS `reinvertOnNumericalTrouble` 流):
    // 数値的原因の再分解のたびに、閾値が既定 (`STABILITY`) 未満なら (既定 on)、または FT 更新 `N` 回未満の分解で起きたら
    // (既定オフ) 閾値を 1 段上げる。
    let pivot_escalation_few_updates: usize = tunable!("ENOMOTO_T_PIVOT_ESCALATE_FEW_UPDATES", PIVOT_ESCALATE_FEW_UPDATES, usize);
    let pivot_escalate_below_default = tunable!("ENOMOTO_T_PIVOT_ESCALATE_BELOW_DEFAULT", PIVOT_ESCALATE_BELOW_DEFAULT, u8) != 0;
    /// 数値的原因による再分解を 1 回記録し、[`PIVOT_ESCALATION_STEP`] 回ごとに LU の
    /// ピボット閾値を引き上げる。(ループ本体が多くの局所変数を可変借用しているため、
    /// クロージャではなくマクロにしている。)
    macro_rules! note_numeric_trouble {
        () => {{
            numeric_trouble_count += 1;
            if pivot_escalation_step != 0 && numeric_trouble_count % pivot_escalation_step == 0 {
                sparse_lu::escalate_pivot_threshold();
            } else if (pivot_escalate_below_default && sparse_lu::pivot_threshold_below_stability()) || (pivot_escalation_few_updates != 0 && lu.update_count() < pivot_escalation_few_updates) {
                sparse_lu::escalate_pivot_threshold();
            }
        }};
    }
    // `d` のドリフト検査用の、より粗い周期のカウンタ(`RESIDUAL_CHECK_MULTIPLIER` 回の
    // FT チェックごとに 1 回)。固定列を除けば真のドリフトは `D_DRIFT_TOL` よりはるかに小さく、
    // `O(n_total)` の BTRAN 付き `fresh_d_into` を毎回払う価値が無いため。
    let mut since_d_drift_check: usize = 0;
    // `super::pivot_values_agree` を無効化するか(`ENOMOTO_DISABLE_UPDATE_VERIFY`)。ループ外で一度だけ読む。
    let update_verify_disabled = env_str!("ENOMOTO_DISABLE_UPDATE_VERIFY").is_some();
    // `ENOMOTO_DEBUG_D_DRIFT_EXT` の診断出力を行うか(ループ外で一度だけ読む)。
    let debug_d_drift_ext = env_str!("ENOMOTO_DEBUG_D_DRIFT_EXT").is_some();
    // `ENOMOTO_PROF_PHASES_EXT`: 主ループのフェーズ別時間計測([`prof_phases`])を行うか。
    let profile_phases = env_str!("ENOMOTO_PROF_PHASES_EXT").is_some();
    if profile_phases {
        prof_phases::reset();
        prof_phases::STAT_M.store(m, std::sync::atomic::Ordering::Relaxed);
    }
    // `ENOMOTO_PROF_PHASES_EXT_WORK=1`(`ENOMOTO_PROF_PHASES_EXT` に追加): 反復あたりの作業量
    // カウンタ(`rho_p`/`alpha`/`tau` の非ゼロ数、PRICE 要素数、触れた列数、候補数)も集める
    // (計測自体が `O(m + PRICE 要素)` かかるので別フラグ)。
    let profile_work = profile_phases && env_str!("ENOMOTO_PROF_PHASES_EXT_WORK").is_some_and(|v| v != "0");
    // `dse_refresh_on_refactor`: 再分解のたびに `DseState::from_basis` で DSE 重みを厳密に
    // 作り直すか(`ENOMOTO_DSE_REFRESH_ON_REFACTOR=1`、既定オフ)。
    let dse_refresh_on_refactor = env_str!("ENOMOTO_DSE_REFRESH_ON_REFACTOR").is_some_and(|v| v != "0");
    // 診断(`ENOMOTO_DEBUG_EXT_DELTA0`): 全 M フラグ列が `M` 側を離れた(delta=0)反復を報告する。
    // 論文の吸収境界の結果(命題 6.10、注意 6.11)により、以後この方法は古典的な有界双対単体法と
    // 一致する。
    let debug_delta0 = env_str!("ENOMOTO_DEBUG_EXT_DELTA0").is_some();
    // 詳細トレース(`ENOMOTO_DEBUG_EXT_TRACE`)を出すか。
    let debug_ext_iters_verbose = env_str!("ENOMOTO_DEBUG_EXT_TRACE").is_some();
    // M フラグ付きで開始時に `M` 側にある構造列の一覧(求解中は不変)。crash が `Zero` に置いた
    // コスト 0 の自由列は値 0 で最初から `M` 側に無く、`Zero` から `M` 側へ戻ることも無い
    // (論文の注意 6.7)ので含めない。
    let m_flagged_cols: Vec<usize> = (0..n_orig).filter(|&j| delta[j].is_flagged() && nb_status[j] != Some(NbStatus::Zero)).collect();
    // 非基底 `Zero` 列の残数: 入基でしか減らないので、0 になれば chuzc1 の `Zero` 判定を省略できる。
    let mut n_zero_nonbasic = nb_status.iter().filter(|s| **s == Some(NbStatus::Zero)).count();
    // delta=0 に初めて到達した反復(診断用)。
    let mut delta0_iter: Option<usize> = None;
    if debug_ext_iters_verbose {
        let one_sided_total = (0..n_orig).filter(|&j| std.lb[j] == f64::NEG_INFINITY || std.ub[j] == f64::INFINITY).count();
        eprintln!("DEBUG_EXT_TRACE: one_sided_unbounded_total={one_sided_total} n_m_flagged(|S|)={} n_zero_start={n_zero_nonbasic}", m_flagged_cols.len());
    }

    // 実験的(`ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL`、未設定なら `1e-9` = 固定許容誤差と同一):
    // M フラグ列が `M` 側を離れるのは入る列 `q` になるときだけ(BFRT フリップでは離れない、
    // 論文の命題 6.10)なので、`resolved_m`/`remaining_m_side` は `q` だけを見ればよい。
    // 未解決の割合から `Score2::cmp_lex` の先頭(傾き)項の許容誤差を決め、多くの M フラグ列が
    // 未解決の間は緩く、delta=0 に近づくにつれ正確な `1e-9` に戻す。さらに M 側の進展が
    // 止まっている間は `stall_shrink`(半減期 `score2_stall_halflife`)で `1e-9` へ引き戻す。
    // `n_m_flagged`: M フラグ列数(0 除算回避のため最小 1)。
    let n_m_flagged = m_flagged_cols.len().max(1);
    // `resolved_m[j]`: 列 `j` がすでに `M` 側を離れた(解決済み)か。`Zero` の列は最初から解決済み。
    let mut resolved_m = vec![false; n_total];
    for j in 0..n_orig {
        if nb_status[j] == Some(NbStatus::Zero) {
            resolved_m[j] = true;
        }
    }
    // まだ `M` 側にある M フラグ列の数(減る一方)。
    let mut remaining_m_side = m_flagged_cols.len();
    // プラトー検出の補助指標: これまでの最小の `remaining_m_side`。`remaining_m_side` は減る一方
    // なので、入れ替わり続ける実行不能行の集合の大きさより確かな進展の指標になる。
    let mut best_remaining_m_side = remaining_m_side;
    // 最後に M フラグ列が解決してからの反復数(`stall_shrink` に使う)。
    let mut iters_since_m_progress: usize = 0;
    // `stall_shrink` の半減期(`ENOMOTO_SCORE2_STALL_HALFLIFE`)。
    let score2_stall_halflife: f64 = env_str!("ENOMOTO_SCORE2_STALL_HALFLIFE")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(SCORE2_STALL_HALFLIFE);
    // `Score2` の傾き項の許容誤差の最大値(`ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL`、既定 `1e-9` = 無効)。
    let score2_max_tol = env_str!("ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(LEX_REL_TOL);
    // 計測用の開始時刻。
    let wall_t0 = std::time::Instant::now();

    // 診断(`ENOMOTO_DEBUG_EXT_DUAL_CHECK`): 双対実行可能性違反を一度だけ報告するための状態と、
    // 直前反復のピボット情報(`q`, `r`, `alpha_q`, `dj_q`, `a_p` の写し・フリップ列・候補数)。
    let debug_dual_check = env_str!("ENOMOTO_DEBUG_EXT_DUAL_CHECK").is_some();
    let mut dual_violation_reported = false;
    let mut prev_q: Option<usize> = None;
    let mut prev_r: Option<usize> = None;
    let mut prev_alpha_q: f64 = 0.0;
    let mut prev_dj_q: f64 = 0.0;
    let mut prev_pivot_debug: Option<(Vec<f64>, Vec<usize>, usize)> = None;

    // 実験的・効果なしと確認済み(`ENOMOTO_STUCK_ROW_BOOST_FACTOR`、未設定/1.0 で無効):
    // `pivot_grossly_inconsistent`/updateVerify がピボットを連続で破棄した行 `stuck_row` について、
    // 連続回数が `stuck_row_boost_threshold` を超えたら、chuzr でその行の逸脱の `M` 係数を
    // `stuck_row_boost_factor` 倍してスコア付けする。
    let mut stuck_row: Option<usize> = None;
    let mut stuck_row_streak: usize = 0;
    // `stuck_row_taboo` で一時的にプールから外したことのある行(デバッグ時の整合性検査で、
    // 新規走査にあってプールに無いことを許す)。
    let mut stuck_row_tabooed = vec![false; m];
    let stuck_row_boost_threshold: usize = env_str!("ENOMOTO_STUCK_ROW_BOOST_THRESHOLD")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(STUCK_ROW_BOOST_THRESHOLD);
    let stuck_row_boost_factor: f64 = env_str!("ENOMOTO_STUCK_ROW_BOOST_FACTOR")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(STUCK_ROW_BOOST_FACTOR);

    // 破棄された候補の禁止(常時オン): `pivot_grossly_inconsistent`/updateVerify がピボットを
    // 破棄したら、その `q` を行 `discard_row` についてだけ禁止し、同じ行が再び選ばれたとき
    // chuzc1 がその列を候補から外す(同じ入力から同じ失敗候補を再び選ぶのを防ぐ)。
    // 別の行のピボットが破棄されるか、`discard_row` のピボットが確定したら解除する。
    // `discard_banned_cols`: 禁止中の列。
    let ban_discarded_candidates = true;
    let mut discard_row: Option<usize> = None;
    let mut discard_banned_cols: Vec<usize> = Vec::new();
    // 診断: 前反復の実行不能行プールの大きさ(増減の集計用)。
    let mut prev_pool_len: Option<usize> = None;
    // S11(実験的・経路を変える・既定オフ): HiGHS `chooseHyperSparse` 風の chuzr 候補短縮リスト。
    // プールの全走査で上位 `K` 行(`shortlist_rows`)と `K+1` 番目のスコア(`shortlist_cut`)を記録し、
    // 以後に逸脱や DSE 重みが変わった行(`x_B` 更新の一覧と `r`)を `shortlist_rows` に追加する。
    // 他の行は `shortlist_cut` 以下のままなので、完全な再同期が挟まらない限り、`shortlist_rows` の最良行が
    // `shortlist_cut` を上回ればそれがプールの最良行。そうでないか、リストが `4K + 64` 行を超えたら
    // 全走査する。`Score2::cmp_lex` の許容誤差は推移的でないのでビット同一ではない。
    // `ENOMOTO_T_CHUZR_SHORTLIST=K`(例: 8)で有効、0 でオフ。
    // 策7: 未指定(0)でも `m >= CHUZR_SHORTLIST_AUTO_MIN_M` の大きな問題では長さ `CHUZR_SHORTLIST_AUTO_K` で
    // 有効にする(実行不能行プールが数万〜十数万行になり、毎反復の全走査が支配的になるため)。
    // 自動のときは全走査のたびにプールの大きさから `K` を決め直す(`shortlist_auto`)。
    // 作業 #10 (A): `CHUZR_HEAP_ADAPTIVE_MIN_M <= m < CHUZR_SHORTLIST_AUTO_MIN_M` でも自動モード (遅延ヒープ) を
    // 有効にし、反復ごとにヒープと全走査の安いほうを選ぶ (`chuzr_heap_adaptive`、下の主ループ参照)。
    let adaptive_min_m = tunable!("ENOMOTO_T_CHUZR_HEAP_ADAPTIVE_MIN_M", CHUZR_HEAP_ADAPTIVE_MIN_M, usize);
    let (mut shortlist_k, shortlist_auto, chuzr_heap_adaptive) = {
        let k = tunable!("ENOMOTO_T_CHUZR_SHORTLIST", CHUZR_SHORTLIST_K, usize);
        let auto_min_m = tunable!("ENOMOTO_T_CHUZR_SHORTLIST_AUTO_MIN_M", CHUZR_SHORTLIST_AUTO_MIN_M, usize);
        let auto_k = tunable!("ENOMOTO_T_CHUZR_SHORTLIST_AUTO_K", CHUZR_SHORTLIST_AUTO_K, usize);
        if k == 0 && auto_min_m > 0 && m >= auto_min_m {
            (auto_k, true, false)
        } else if BIG && k == 0 && adaptive_min_m > 0 && m >= adaptive_min_m && tunable!("ENOMOTO_T_CHUZR_HEAP", 1u8, u8) != 0 {
            (auto_k, true, true)
        } else {
            (k, false, false)
        }
    };
    let shortlist_enabled = shortlist_k > 0 && merge_flip_xb && score2_max_tol == LEX_REL_TOL && stuck_row_boost_factor == 1.0;
    // 短縮リストの行一覧、リスト所属の印、全走査時の上位 `K+1` 行、カットのスコア、有効か。
    let mut shortlist_rows: Vec<usize> = Vec::new();
    let mut in_shortlist = vec![false; if shortlist_enabled { m } else { 0 }];
    let mut shortlist_top: Vec<(Score2, usize)> = Vec::with_capacity(shortlist_k + 2);
    let mut shortlist_cut: Option<Score2> = None;
    let mut shortlist_valid = false;
    // 策7(自動モード): 短縮リストの代わりに、プール全行のスコアを持つ遅延最大ヒープで chuzr を行う
    // (`ChuzrEntry`)。逸脱か DSE 重みが変わった行(`x_B` 更新の一覧と `r`)は版番号 `chuzr_ver` を
    // 進めて新しい要素を積み、古い要素は取り出し時に捨てる。完全な再同期(再分解など)や一覧のない
    // 反復の後は作り直す。`ENOMOTO_T_CHUZR_HEAP=0` で自動モードも S11 の短縮リストを使う。
    let chuzr_heap_mode = BIG && shortlist_enabled && shortlist_auto && tunable!("ENOMOTO_T_CHUZR_HEAP", 1u8, u8) != 0;
    let mut chuzr_heap: BinaryHeap<ChuzrEntry> = BinaryHeap::new();
    let mut chuzr_ver: Vec<u32> = vec![0; if chuzr_heap_mode { m } else { 0 }];
    // 前反復の終わりにヒープがプールを正しく表していたか。
    let mut chuzr_heap_valid = false;
    // square41 / ex10 報告の策7: 前反復の `x_B` 更新が一覧なし (全行走査 = `alpha` が密) だったら、この反復は
    // 遅延ヒープを作り直さずプール全体の走査で選ぶ。ex10 (`alpha` の 80% が非ゼロ) では毎反復ヒープが
    // 無効になり、プール 2 万行の `BinaryHeap::from` を毎反復払っていた (走査は同じ `O(プール)` でも
    // 比較 1 回/行で済む)。選ぶ行は `cmp_lex` の許容誤差の非推移性のぶんヒープと違いうる (経路が変わる)。
    // 次に一覧のある反復が来たらヒープを作り直す。`ENOMOTO_T_CHUZR_DENSE_SCAN=0` で無効。
    let chuzr_dense_scan = chuzr_heap_mode && tunable!("ENOMOTO_T_CHUZR_DENSE_SCAN", 1u8, u8) != 0;
    let mut chuzr_scan_next = false;
    // 作業 #10 (A): 適応モード (`chuzr_heap_adaptive`) で反復ごとにヒープを使うかの判定の係数と、
    // 前反復の `x_B` 更新の一覧の長さ (ヒープに積む要素数の見積り)。ヒープの 1 反復の手間は
    // 「積む要素数 × log2(プール)」、全走査は「プール行数」に比例するので、
    // `プール >= CHUZR_HEAP_ADAPTIVE_RATIO * (一覧長 + 1) * log2(プール)` のときだけヒープを使う。
    let chuzr_heap_ratio = if chuzr_heap_adaptive { tunable!("ENOMOTO_T_CHUZR_HEAP_ADAPTIVE_RATIO", CHUZR_HEAP_ADAPTIVE_RATIO, f64) } else { 0.0 };
    let mut chuzr_prev_list_len: usize = usize::MAX;
    // `x_B(M)` のドリフト検査(と eta フィル検査)の反復間隔。策9: `m >= XB_CHECK_CADENCE_LARGE_M` の
    // 大きな問題では `max(XB_CHECK_CADENCE_LARGE, m / XB_CHECK_CADENCE_LARGE_DIV)` に伸ばす。
    let xb_check_cadence = {
        let large_m = tunable!("ENOMOTO_T_XB_CHECK_LARGE_M", XB_CHECK_CADENCE_LARGE_M, usize);
        if large_m > 0 && m >= large_m {
            tunable!("ENOMOTO_T_XB_CHECK_CADENCE_LARGE", XB_CHECK_CADENCE_LARGE, usize).max(m / tunable!("ENOMOTO_T_XB_CHECK_CADENCE_LARGE_DIV", XB_CHECK_CADENCE_LARGE_DIV, usize).max(1))
        } else {
            tunable!("ENOMOTO_T_XB_CHECK_INTERVAL", XB_CHECK_CADENCE, usize)
        }
    };
    // 主ループ内の再分解。特異なら(作業 #5 M3)直近の成功した再分解以降のピボットを巻き戻して
    // 前の基底を分解し直し([`rollback_pivots`])、戻したピボットの `(r, q)` を以後の比率テストで禁止する
    // (`rb_bans`、`filter_banned_pivots`)。巻き戻せなければ(または `ROLLBACK_MAX` 回を超えたら)従来どおり
    // `None`(`NotSolved`、安全モードで解き直し)。
    macro_rules! refactor_main {
        () => {{
            match refactorize(std, &basis_pos, Some(&lu)) {
                Some(l) => {
                    pivot_log.clear();
                    pivot_flips.clear();
                    l
                }
                None => {
                    if !pivot_rollback {
                        return None;
                    }
                    let (l, bans) = rollback_pivots(
                        std,
                        &cache,
                        &noise_feasible,
                        &mut basis,
                        &mut basis_pos,
                        &mut nb_status,
                        &mut row_bounds,
                        &mut pivot_log,
                        &mut pivot_flips,
                        price_nonbasic_only,
                        &mut price_col,
                        &mut price_val,
                        &mut col_entry_of_price,
                        &mut price_pos_of_col_entry,
                        &mut price_nb_end,
                        &col_entry_start,
                        &price_start,
                        &mut n_zero_nonbasic,
                        &lu,
                    )?;
                    diag_rollbacks += 1;
                    let (r0, q0) = bans[0];
                    if noise_diag {
                        eprintln!("DEBUG_EXT: singular refactor -> rolled back pivots (last r={r0} q={q0}, banned {}), rollbacks={diag_rollbacks}", bans.len());
                    }
                    if diag_rollbacks > ROLLBACK_MAX {
                        return None;
                    }
                    pivot_log.clear();
                    pivot_flips.clear();
                    stuck_row = Some(r0);
                    stuck_row_streak = 1;
                    // 戻したピボットの `(r, q)` を以後の比率テストで禁止する(反復をまたいで残す。同じ特異化の繰り返しを防ぐ)。
                    for &(rb, qb) in &bans {
                        rb_ban_row[rb] = true;
                        if !rb_bans.contains(&(rb, qb)) {
                            rb_bans.push((rb, qb));
                        }
                    }
                    shortlist_valid = false;
                    chuzr_heap_valid = false;
                    l
                }
            }
        }};
    }
    // 作業 #5 M2: 実行不能を結論する直前のガード。行 `r` の逸脱 `w_r` が雑音水準(M1 の式、重みの下限なし)か
    // `sqrt(w_r)` が [`INFEAS_GUARD_SQRT_W`] を超えるなら、PRICE 行は雑音で結論できない。
    macro_rules! infeas_guard {
        ($r:expr, $w_r:expr) => {{
            let wr = dse.weight($r);
            let w_r: Affine1 = $w_r;
            noise_on && (wr > infeas_guard_w || (w_r.slope == 0.0 && w_r.base * w_r.base <= noise_row_k2 * wr))
        }};
    }
    // 作業 #5 M2: 実行不能を結論する直前(`update_count == 0`)の検査。(1) `infeas_guard!` が成り立てば行を一時的に
    // 外して続ける。(2) 段階 B では `rho` による Farkas の証明([`infeasibility_certified`])が成り立つときだけ結論する
    // (段階 A は実行不能を証明しない)。成り立たなければ LU の閾値を 1 段上げて再分解・再同期し、上限なら行を外す。
    // 証明できない結論が `UNCERTIFIED_MAX` 回を超えたら `None`(`NotSolved`)。
    macro_rules! infeas_check {
        ($site:expr, $iter:expr, $r:expr, $w_r:expr) => {{
            let r: usize = $r;
            let w_r: Affine1 = $w_r;
            if noise_on || infeas_certify {
                let guard = infeas_guard!(r, w_r);
                let certified = !guard
                    && phase != Phase::A
                    && infeasibility_certified(std, &basis_pos, &nb_status, &rho, r, |j| (affine_bound_value(cache.lower[j], f64::NEG_INFINITY), affine_bound_value(cache.upper[j], f64::INFINITY)));
                if !certified {
                    diag_infeas_guard += 1;
                    if noise_diag {
                        eprintln!("DEBUG_EXT: infeas_guard site={} iter={} r={r} sqrt_w={:.3e} dev=({:.3e},{:.3e}) guard={guard} count={diag_infeas_guard} degenerate_pivots={diag_degen} max_run={diag_degen_max_run} cost_shifts={diag_shift_events}", $site, $iter, dse.weight(r).sqrt(), w_r.base, w_r.slope);
                    }
                    if diag_infeas_guard > UNCERTIFIED_MAX {
                        return None;
                    }
                    if !guard && sparse_lu::escalate_pivot_threshold() {
                        lu = refactor_main!();
                        let (fresh_base, fresh_slope) = resync_x_b(std, &cache, &nb_status, &lu, phase, &mut lu_scratch, &mut x_b_base, &mut x_b_slope)?;
                        rhs_inc_base = fresh_base;
                        rhs_inc_slope = fresh_slope;
                        rebuild_rows(&mut infeasible_rows, &mut row_dev, m, &row_bounds, &x_b_base, &x_b_slope);
                        fresh_d_into(std, &lu, &basis, &basis_pos, &active_cost, &mut fresh_d_cb, &mut lu_scratch, &mut fresh_d_y, &mut d);
                        continue;
                    }
                    // 作業 #8 対処 5: 閾値を上げきっても証明が立たない(基底が数値的に壊れている)なら、最初の求解では
                    // 行を外して壊れた基底で続けず、摂動を掛け直して安全モードで最初から解き直す(`solve_slope_intercept_dual`)。
                    if !guard && !safe_pivot && uncertified_restart {
                        RESTART_BAILOUT.with(|f| f.set(true));
                        if noise_diag {
                            eprintln!("DEBUG_EXT_BAILOUT: uncertified infeasibility at iter={} -> restart with re-perturbed costs", $iter);
                        }
                        return None;
                    }
                    infeasible_rows.set(r, false);
                    stuck_row_tabooed[r] = true;
                    continue;
                }
            }
        }};
    }
    // ===== 主ループ(1 反復 = chuzr → BTRAN → PRICE → chuzc1/BFRT → FTRAN → 更新) =====
    for iter_idx in 0..max_iters {
        // 前反復終了時点で候補短縮リストが有効だったか(この反復では一旦無効にする)。
        let shortlist_was_valid = shortlist_valid;
        shortlist_valid = false;
        let chuzr_heap_was_valid = chuzr_heap_valid;
        chuzr_heap_valid = false;
        // 策7 (ex10): この反復は遅延ヒープではなく全走査で選ぶか。
        let mut chuzr_scan_now = chuzr_scan_next && !chuzr_heap_was_valid;
        chuzr_scan_next = false;
        // 作業 #10 (A): 適応モードでは、プールが小さい (または前反復の一覧が長い) 反復は全走査で選ぶ。
        if BIG && chuzr_heap_adaptive && !chuzr_scan_now {
            let pool = infeasible_rows.rows.len();
            let heap_cost = chuzr_heap_ratio * (chuzr_prev_list_len.saturating_add(1)) as f64 * (pool.max(2) as f64).log2();
            if (pool as f64) < heap_cost {
                chuzr_scan_now = true;
            }
        }
        // この反復の chuzr を遅延ヒープで行ったか(反復の終わりに変わった行を積めば有効なまま保てる)。
        let mut chuzr_heap_ready = false;
        // この反復の chuzr 時点で `shortlist_rows`/`shortlist_cut` がプールを正しく表しているか
        // (短縮リストが当たったか、全走査で作り直したか)。
        let mut shortlist_ready = false;
        iters_since_m_progress += 1;
        if debug_ext_iters_verbose && iter_idx % 2000 == 0 {
            eprintln!(
                "DEBUG_EXT_TRACE: iter={iter_idx} infeasible_rows={} stall_count={stall_count} bland_mode={bland_mode} remaining_m_side={remaining_m_side}",
                infeasible_rows.rows.len()
            );
        }
        if profile_phases {
            prof_phases::ITERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let pool_len = infeasible_rows.rows.len();
            prof_phases::INFEASIBLE_POOL.fetch_add(pool_len, std::sync::atomic::Ordering::Relaxed);
            if let Some(prev) = prev_pool_len {
                use std::cmp::Ordering as CmpOrdering;
                match pool_len.cmp(&prev) {
                    CmpOrdering::Greater => prof_phases::POOL_GREW.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    CmpOrdering::Less => prof_phases::POOL_SHRANK.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    CmpOrdering::Equal => prof_phases::POOL_SAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                };
            }
            prev_pool_len = Some(pool_len);
        }
        // 診断(`ENOMOTO_DEBUG_EXT_DUAL_CHECK`): 最初に現れた大きな双対実行可能性違反を
        // 直前反復のピボット情報とともに一度だけ報告する。
        if debug_dual_check && !dual_violation_reported {
            let mut worst: Option<(usize, f64)> = None;
            for j in 0..std.n_total {
                let Some(status) = nb_status[j] else { continue };
                if std.lb[j] == std.ub[j] {
                    continue;
                }
                let dj = d[j];
                let viol = match status {
                    NbStatus::Lower => -dj,
                    NbStatus::Upper => dj,
                    NbStatus::Zero => dj.abs(),
                };
                if viol > 1.0 && worst.map_or(true, |(_, w)| viol > w) {
                    worst = Some((j, viol));
                }
            }
            if let Some((j, viol)) = worst {
                eprintln!(
                    "DEBUG_EXT_DUAL_CHECK: first dual-feasibility violation (>1.0) at iter={iter_idx} j={j} magnitude={viol} -- caused by PREVIOUS iter's pivot: prev_q={prev_q:?} prev_r={prev_r:?} prev_alpha_q={prev_alpha_q} prev_dj_q={prev_dj_q}"
                );
                if let Some((ap, flips, ncand)) = &prev_pivot_debug {
                    let flipped = flips.contains(&j);
                    eprintln!(
                        "  DBG j={j}: prev a_p[j]={} flipped_prev={flipped} n_flips={} n_cand={ncand} status={:?} d[j]={} delta={:?} lower={:?} upper={:?} is_slack={}",
                        ap[j], flips.len(), nb_status[j], d[j], if j < n_orig { delta[j] } else { MSide::None }, cache.lower[j], cache.upper[j], j >= n_orig
                    );
                    let mut sumabs = 0.0f64;
                    for (i, &v) in ap.iter().enumerate() { if v.abs() > 1e3 { sumabs += 1.0; let _ = i; } }
                    eprintln!("  DBG prev row has {sumabs} entries with |a_p|>1e3");
                }
                dual_violation_reported = true;
            }
        }
        // (a')(b')(論文 6.1 節): 主実行可能性の確認と離基行の選択。DSE 重み付き(5.4 節 Step 2(b) の一般化、
        // `Score2`)で、`infeasible_rows.rows`(主実行不能な行)だけを走査する。
        // `score2_c2_tol`: `Score2` の先頭(傾き)項の相対許容誤差。M フラグ列が全て未解決のとき
        // `score2_max_tol`、`remaining_m_side == 0`(delta=0)のとき基準値 `LEX_REL_TOL` になるよう
        // 線形補間し、さらに M 側の進展が止まっている間は `stall_shrink` で基準値側へ引き戻す。
        // `.max(LEX_REL_TOL)` で基準より厳しくはしない。既定(`score2_max_tol = LEX_REL_TOL`)では常に基準値。
        let stall_shrink = score2_stall_halflife / (score2_stall_halflife + iters_since_m_progress as f64);
        let score2_c2_tol = (LEX_REL_TOL + (score2_max_tol - LEX_REL_TOL) * (remaining_m_side as f64 / n_m_flagged as f64) * stall_shrink).max(LEX_REL_TOL);
        // 選ばれた離基行 `(行, 方向 d_dir, 逸脱 dev, スコア)`。
        let mut best: Option<(usize, i32, Affine1, Score2)> = None;
        // この反復で候補短縮リスト(S11)を使うか(遅延ヒープの自動モードでは使わない)。
        let shortlist_active = shortlist_enabled && !bland_mode && !chuzr_heap_mode;
        if chuzr_heap_mode && !bland_mode && !chuzr_scan_now {
            timed!(profile_phases, prof_phases::CHUZR, {
                let pool = infeasible_rows.rows.len();
                // 作り直しが要るか(前反復で無効になった、または古い要素がたまりすぎた)。
                let mut rebuild = !chuzr_heap_was_valid || chuzr_heap.len() > 2 * pool + 1024;
                loop {
                    if rebuild {
                        let mut v = std::mem::take(&mut chuzr_heap).into_vec();
                        v.clear();
                        v.extend(infeasible_rows.rows.iter().map(|&i| ChuzrEntry { score: Score2::new(row_dev.dev[i], dse.weight(i)), row: i as u32, ver: chuzr_ver[i] }));
                        chuzr_heap = BinaryHeap::from(v);
                    }
                    // 古い要素(版番号が違う、またはプールを外れた行)を捨てる。
                    while let Some(top) = chuzr_heap.peek() {
                        let i = top.row as usize;
                        if top.ver == chuzr_ver[i] && infeasible_rows.contains(i) {
                            break;
                        }
                        chuzr_heap.pop();
                    }
                    if chuzr_heap.is_empty() && pool > 0 && !rebuild {
                        rebuild = true;
                        continue;
                    }
                    break;
                }
                best = chuzr_heap.peek().map(|top| {
                    let i = top.row as usize;
                    (i, row_dev.dir[i], row_dev.dev[i], top.score)
                });
            });
            shortlist_ready = true;
            chuzr_heap_ready = true;
        }
        // 短縮リストが有効でプールが十分大きければ、リスト内の最良行がカットを超えるかを試す。
        if shortlist_active && shortlist_was_valid && infeasible_rows.rows.len() > CHUZR_SHORTLIST_MIN_POOL_FACTOR * shortlist_k {
            timed!(profile_phases, prof_phases::CHUZR, {
                let mut b: Option<(usize, i32, Affine1, Score2)> = None;
                for &i in &shortlist_rows {
                    if !infeasible_rows.contains(i) {
                        continue;
                    }
                    let (d_dir, dev) = (row_dev.dir[i], row_dev.dev[i]);
                    let score = Score2::new(dev, dse.weight(i));
                    let better = match b {
                        None => true,
                        Some((br, _, _, bscore)) => match score.cmp_lex(&bscore, score2_c2_tol) {
                            std::cmp::Ordering::Greater => true,
                            std::cmp::Ordering::Less => false,
                            std::cmp::Ordering::Equal => i < br,
                        },
                    };
                    if better {
                        b = Some((i, d_dir, dev, score));
                    }
                }
                if let Some(bb) = b {
                    if shortlist_cut.map_or(true, |c| bb.3.cmp_lex(&c, score2_c2_tol) == std::cmp::Ordering::Greater) {
                        best = b;
                        shortlist_ready = true;
                    }
                }
            });
        }
        // 短縮リストで決まらなければプール全体を走査する(必要なら短縮リストも作り直す)。
        if !shortlist_ready {
        timed!(profile_phases, prof_phases::CHUZR, {
            if shortlist_active {
                // 短縮リストを同じ走査で作り直す: プール中の上位 `K + 1` 行を、最も劣る行を根に持つ
                // 二分ヒープ(`shortlist_top`)に集める。策7 の自動モードでは `K` をプールの大きさから決める。
                if shortlist_auto {
                    shortlist_k = (infeasible_rows.rows.len() / tunable!("ENOMOTO_T_CHUZR_SHORTLIST_AUTO_DIV", CHUZR_SHORTLIST_AUTO_DIV, usize).max(1))
                        .clamp(tunable!("ENOMOTO_T_CHUZR_SHORTLIST_AUTO_K", CHUZR_SHORTLIST_AUTO_K, usize), tunable!("ENOMOTO_T_CHUZR_SHORTLIST_AUTO_K_MAX", CHUZR_SHORTLIST_AUTO_K_MAX, usize).max(1));
                }
                shortlist_top.clear();
            }
            for &i in &infeasible_rows.rows {
                // 行のメンバーシップを最後に判定した時点でキャッシュした逸脱(`RowDevCache`)。
                let (d_dir, dev) = (row_dev.dir[i], row_dev.dev[i]);
                // `stuck_row` が指す行だけ、順位付け用のスコアで逸脱の `M` 係数を
                // `stuck_row_boost_factor` 倍する(`dev` 自体は真の値のまま)。
                let scored_dev = if stuck_row_streak >= stuck_row_boost_threshold && stuck_row == Some(i) {
                    Affine1::new(dev.base, dev.slope * stuck_row_boost_factor)
                } else {
                    dev
                };
                let score = Score2::new(scored_dev, dse.weight(i));
                let better = match best {
                    None => true,
                    Some((br, _, _, bscore)) => {
                        if bland_mode {
                            i < br
                        } else {
                            match score.cmp_lex(&bscore, score2_c2_tol) {
                                std::cmp::Ordering::Greater => true,
                                std::cmp::Ordering::Less => false,
                                std::cmp::Ordering::Equal => i < br,
                            }
                        }
                    }
                };
                if better {
                    best = Some((i, d_dir, dev, score));
                }
                if shortlist_active {
                    // `shortlist_enabled` はブースト無効が条件なので `score` はブーストなしのスコア。
                    shortlist_heap_offer(&mut shortlist_top, shortlist_k + 1, (score, i), score2_c2_tol);
                }
            }
            if shortlist_active {
                for &i in &shortlist_rows {
                    in_shortlist[i] = false;
                }
                shortlist_rows.clear();
                // ヒープが `K + 1` 行で満杯なら根(最も劣る行)がカット、残り `K` 行がリスト。
                let full = shortlist_top.len() > shortlist_k;
                shortlist_cut = if full { Some(shortlist_top[0].0) } else { None };
                for &(_, i) in shortlist_top.iter().skip(usize::from(full)) {
                    in_shortlist[i] = true;
                    shortlist_rows.push(i);
                }
                shortlist_ready = true;
            }
        });
        }
        #[cfg(debug_assertions)]
        {
            // `InfeasibleRows` と `RowDevCache` が全行の新規走査結果と一致することを確認する。
            let mut fresh: Vec<usize> = (0..m).filter(|&i| !stuck_row_tabooed[i] && row_infeasible_affine(&cache, &basis, &x_b_base, &x_b_slope, &noise_feasible, i)).collect();
            let mut maintained: Vec<usize> = infeasible_rows.rows.iter().copied().filter(|&i| !stuck_row_tabooed[i]).collect();
            fresh.sort_unstable();
            maintained.sort_unstable();
            debug_assert_eq!(fresh, maintained, "InfeasibleRows drifted from a fresh scan at iter {iter_idx}");
            for &i in &infeasible_rows.rows {
                let (fd, fv) = row_deviation(&cache, &basis, &x_b_base, &x_b_slope, &noise_feasible, i).expect("pool row must be infeasible");
                let cv = row_dev.dev[i];
                debug_assert!(
                    fd == row_dev.dir[i] && fv.base.to_bits() == cv.base.to_bits() && fv.slope.to_bits() == cv.slope.to_bits(),
                    "RowDevCache drifted from a fresh row_deviation at iter {iter_idx}, row {i}"
                );
            }
        }
        // 以後はスコアを落とした `(行, 方向, 逸脱)` だけを使う。
        let mut best = best.map(|(i, d_dir, dev, _)| (i, d_dir, dev));

        // 「最大改善」chuzr エスカレーション(`GREATEST_IMPROVEMENT_TOP_K` 参照): DSE による選択が
        // しばらく停滞したときだけ、DSE 上位の候補行を試行 PRICE による目的関数改善量の推定で
        // 並べ直す。
        if greatest_improvement_enabled && !bland_mode && stall_count > greatest_improvement_stall_threshold && infeasible_rows.rows.len() > 1 {
            timed!(profile_phases, prof_phases::CHUZR, {
                // スコア降順の上位 `GREATEST_IMPROVEMENT_TOP_K` 件を安定挿入で保つ(同点は先に来た行が前)。
                // `cmp_lex` は許容誤差つきで推移律を満たさないことがあり、`sort_by` に渡すと
                // 標準ライブラリが順序違反を検出して panic する(klein2)。挿入なら panic せず、
                // 比較が整合している入力では安定ソート + truncate と同じ結果になる。
                greatest_improvement_cands.clear();
                for &i in &infeasible_rows.rows {
                    let (d_dir_i, dev_i) = (row_dev.dir[i], row_dev.dev[i]);
                    let score = Score2::new(dev_i, dse.weight(i));
                    let pos = greatest_improvement_cands.iter().position(|t| score.cmp_lex(&t.3, score2_c2_tol) == std::cmp::Ordering::Greater);
                    match pos {
                        Some(p) => {
                            greatest_improvement_cands.insert(p, (i, d_dir_i, dev_i, score));
                            greatest_improvement_cands.truncate(GREATEST_IMPROVEMENT_TOP_K);
                        }
                        None if greatest_improvement_cands.len() < GREATEST_IMPROVEMENT_TOP_K => greatest_improvement_cands.push((i, d_dir_i, dev_i, score)),
                        None => {}
                    }
                }

                let mut best_gain: Option<Affine1> = None;
                let mut best_gi: Option<(usize, i32, Affine1)> = None;
                for &(i, d_dir_i, dev_i, _) in &greatest_improvement_cands {
                    // `rho` を全体で書くので、超疎 BTRAN の出力位置の記録を無効化する。
                    if BIG {
                        btran_work.invalidate_out();
                    }
                    let Some(ratio) = trial_row_ratio(std, &lu, &nb_status, &d, d_dir_i, &mut lu_scratch, &mut rho, &mut a_p, &mut touched, &mut touched_cols, i) else {
                        continue;
                    };
                    let gain = dev_i.scale(ratio);
                    let better = match best_gain {
                        None => true,
                        Some(bg) => gain.cmp_lex(&bg) == std::cmp::Ordering::Greater,
                    };
                    if better {
                        best_gain = Some(gain);
                        best_gi = Some((i, d_dir_i, dev_i));
                    }
                }
                if let Some(chosen) = best_gi {
                    if debug_greatest_improvement {
                        eprintln!(
                            "DEBUG_EXT_GREATEST_IMPROVEMENT: iter={iter_idx} stall_count={stall_count} dse_pick={:?} gi_pick={} gain=({},{})",
                            best.map(|(i, ..)| i),
                            chosen.0,
                            best_gain.unwrap().base,
                            best_gain.unwrap().slope
                        );
                    }
                    best = Some(chosen);
                }
            });
        }

        // 主実行不能な行が無い: 段階 A なら段階 B へ移行し、そうでなければ後処理(Algorithm 1)へ進む。
        let Some((r, d_dir, w_r)) = best else {
            if phase == Phase::A {
                // 段階 A → B の移行(論文 Algorithm 1): 傾き問題が最適になったので
                // `x^1`(したがって `z^1` と切片問題の全境界)は以後固定(論文の命題 7.2 (ii))。
                // 段階 B の境界に差し替え、傾きチャネルを 0 に固定し、切片 `x^0` を一度だけ解く。
                // 基底・`d`・DSE 重みはそのまま引き継ぐ(`c` と `B` は両問題で共通)。
                // `z^1 = c^T x^1` は真の(摂動なし)コストで計算する(`finish` の判定と同じ量)。
                let z1: f64 = (0..n_total)
                    .map(|j| {
                        let s = match (basis_pos[j], nb_status[j]) {
                            (Some(pos), _) => x_b_slope[pos],
                            (None, Some(NbStatus::Lower)) => cache.lower[j].map_or(0.0, |a| a.slope),
                            (None, Some(NbStatus::Upper)) => cache.upper[j].map_or(0.0, |a| a.slope),
                            _ => 0.0,
                        };
                        std.c[j] * s
                    })
                    .sum();
                if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
                    eprintln!("DEBUG_EXT: stage_a_iters={iter_idx} z1={z1}");
                }
                // `z^1 < 0`: 実行不能か非有界で、有限最適は無い(論文の系 7.3 (i))。どちらかの
                // 区別を呼び出し側が求めていなければここで `InfeasibleOrUnbounded` を返す。
                // (`z^1 = 0` なら有限最適か実行不能で、どちらにせよ段階 B が必要。)
                if z1 < -Z_SLOPE_TOL && !opts.distinguish_infeasible_unbounded {
                    if profile_phases {
                        prof_phases::report(wall_t0.elapsed().as_nanos() as usize);
                    }
                    return Some(SimplexResult { status: Status::InfeasibleOrUnbounded, x: None });
                }
                cache = ColCache::intercept_problem(&cache_orig, &nb_status, &basis, &x_b_slope)?;
                width_inf = cache.width.iter().map(|w| w.is_none()).collect();
                row_bounds = RowBounds::new(&cache, &basis, &noise_feasible);
                phase = Phase::B;
                // 作業 #5 M3: 段階をまたいで巻き戻さない(境界 `cache` が替わるため)。
                pivot_log.clear();
                pivot_flips.clear();
                x_b_slope.fill(0.0);
                let (fresh_base, fresh_slope) = resync_x_b(std, &cache, &nb_status, &lu, phase, &mut lu_scratch, &mut x_b_base, &mut x_b_slope)?;
                rhs_inc_base = fresh_base;
                rhs_inc_slope = fresh_slope;
                rebuild_rows(&mut infeasible_rows, &mut row_dev, m, &row_bounds, &x_b_base, &x_b_slope);
                // 新しい LP の開始: 巡回防止・停滞の履歴をリセットする。
                stall_count = 0;
                bland_mode = false;
                infeasible_plateau_count = 0;
                best_infeasible_len = infeasible_rows.rows.len();
                continue;
            }
            // M 切り詰め問題で主実行可能(論文 6.1 節 (a') の `V_∞ = ∅`、段階 B の最適): 後処理へ。
            if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
                eprintln!("DEBUG_EXT: main_loop_iters={iter_idx} bland_mode={bland_mode}");
            }
            if debug_delta0 {
                match delta0_iter {
                    Some(k) => eprintln!(
                        "DEBUG_EXT_DELTA0: delta=0 first reached at iter={k}, main_loop_iters={iter_idx}, iters_after_delta0={}",
                        iter_idx - k
                    ),
                    None => eprintln!("DEBUG_EXT_DELTA0: delta never reached 0 within the main loop (main_loop_iters={iter_idx})"),
                }
            }
            if profile_phases {
                prof_phases::report(wall_t0.elapsed().as_nanos() as usize);
            }
            if noise_diag {
                eprintln!("DEBUG_EXT: degenerate_pivots={diag_degen} max_run={diag_degen_max_run} cost_shifts={diag_shift_events} shifted_cols={diag_shift_cols} xb_mismatch={:.3e}@{}", diag_xb_mismatch.0, diag_xb_mismatch.1);
            }
            if noise_diag && noise_on {
                eprintln!(
                    "DEBUG_EXT: noise max_pick_sqrt_w={:.3e} noise_rows={diag_noise_rows} noise_pivot_iters={diag_noise_pivot_iters} noise_pivot_cands={diag_noise_pivot_cands} noise_taboo={diag_noise_taboo} infeas_guard={diag_infeas_guard} rollbacks={diag_rollbacks}",
                    diag_max_pick_w.sqrt()
                );
            }
            return finish(std, &mut basis, &mut basis_pos, &mut nb_status, &delta, &cache_orig, n_orig, lu);
        };
        // 作業 #5 M1: 選んだ行の逸脱が、その行の主値の丸め誤差の水準 `C eps sqrt(w_r) max(‖b‖∞, 1)` 以下なら
        // 雑音として以後実行可能扱いにする(`Eligible = ∅` の `noise_feasible` と同じ処理)。基底が数値的に従属に
        // 近い行(`sqrt(w_r)` 巨大)で逸脱 1e-9 級の雑音を「実行不能」と選び、雑音の PRICE 行にピボットして基底を
        // 特異にするのを防ぐ。`sqrt(w_r) <= NOISE_MIN_SQRT_W` の行には適用しない(通常の問題の経路を変えない)。
        if noise_on {
            let wr = dse.weight(r);
            if noise_diag && wr > diag_max_pick_w {
                diag_max_pick_w = wr;
            }
            if wr > noise_min_w && w_r.slope == 0.0 && w_r.base * w_r.base <= noise_row_k2 * wr {
                diag_noise_rows += 1;
                if !noise_dry {
                    noise_feasible[basis[r]] = true;
                    row_bounds.mark_noise(r);
                    infeasible_rows.set(r, false);
                    continue;
                }
            }
        }

        // (c): ピボット行の BTRAN `rho = B^-T e_r`(`M` に依存しないので `rho`/`a_p`/`d` は
        // 通常の `f64`)。この反復の `try_update_precomputed` 用に `e_tilde_buf`
        // (U^-T 後・R 逆適用前の中間値)を副産物としてキャプチャする。
        timed!(profile_phases, prof_phases::BTRAN, {
            if BIG {
                lu.solve_transpose_unit_work_sparse(r, &mut rho, &mut e_tilde_buf, &mut btran_work, Some(&mut rho_steps))
            } else {
                lu.solve_transpose_unit_work(r, &mut rho, &mut e_tilde_buf, &mut btran_work, Some(&mut rho_steps))
            }
        });
        // 診断: 維持している DSE 重みと `‖rho‖^2`(真の値)の相対誤差を区間別に数える
        // (`O(m)` なので作業量集計 `ENOMOTO_PROF_PHASES_EXT_WORK` のときだけ。策8)。
        if profile_work {
            let exact_w = dot(&rho, &rho);
            let maintained_w = dse.weight(r);
            let rel_err = (maintained_w - exact_w).abs() / exact_w.max(1e-9);
            prof_phases::DSE_CHECKED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let bucket = if rel_err < 0.01 {
                &prof_phases::DSE_ERR_LT_1PCT
            } else if rel_err < 0.10 {
                &prof_phases::DSE_ERR_LT_10PCT
            } else if rel_err < 1.00 {
                &prof_phases::DSE_ERR_LT_100PCT
            } else {
                &prof_phases::DSE_ERR_GE_100PCT
            };
            bucket.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        // 行方向の疎 PRICE(Huangfu & Hall §2.2.2): `rho` の非ゼロ行だけを走査して
        // `a_p = rho^T A` を作る。`a_p` は `d` の増分更新にも使う。固定列だけを除外する
        // (`price_nonbasic_only` のときは基底列も分割で除外され、離基列の `d` は直接設定する)。
        // S9(`price_by_column`、既定オフ): `rho` が密なら列ごとの内積(gather)で求める。
        // 加算順が変わるので経路は変わりうる。
        // S9 の判定: `rho` の非ゼロ数と、それが `price_column_density * m` を超えるか。
        let rho_count_for_col = if price_by_column { rho.iter().filter(|v| v.abs() > TOL).count() } else { 0 };
        let price_by_col_now = price_by_column && (rho_count_for_col as f64) > price_column_density * m as f64;
        if price_by_col_now {
            timed!(profile_phases, prof_phases::PRICE, {
                for &j in &price_col_list {
                    let j = j as usize;
                    if price_nonbasic_only && nb_status[j].is_none() {
                        continue;
                    }
                    let mut acc = 0.0f64;
                    for &(i, v) in std.cols.col(j) {
                        acc += rho[i] * v;
                    }
                    if acc != 0.0 {
                        a_p[j] = acc;
                        touched[j] = true;
                        touched_cols.push(j);
                    }
                }
            });
        }

        // S8: `rho` が疎と予測される(前反復の `|rho_i| > TOL` の個数が
        // `ENOMOTO_T_PRICE_LIST_DENSITY * m` 以下)なら、非ゼロ行を昇順に圧縮してそれだけを走査する
        // (同じ行を同じ順に訪れるので `a_p` と `touched_cols` の順序はビット単位で同一)。
        // この一覧(正確な非ゼロ)は DSE 更新の `Σ rho_i^2` にも使う。
        // `rho_list_len`: 圧縮した一覧の長さ(全行走査なら `None`)。
        let mut rho_list_len: Option<usize> = None;
        // square41 報告の策3: 前反復の PRICE 要素数が `price_dense_ratio · n_total` を超えたら密結果モード
        // (`rho` が密で、ほぼ全列に触れる)。PRICE の各要素での初到達判定 (`a_p` の読みに依存する分岐) と
        // `touched_cols` への積み込みをやめ、加算後に `a_p` を 1 回順に走査して「触れた」列 (`a_p` のビットが
        // `+0.0` でない列) を列番号の昇順で一覧にする。`a_p` の値と触れた列の集合は同じで、
        // 一覧の順序は後段 (chuzc1 の候補は `(ratio, j)` の全順序で並べる、`d` 更新は列ごとに独立) に
        // 影響しないのでビット一致。加算の結果がちょうど 0 になった列は `-0.0` を置いて「触れた」印を残す
        // (`-0.0 + x` は `+0.0 + x` と同じ値。`-0.0` を読むのは chuzc1 (`|alpha_j| <= TOL` で落ちる) と
        // `d` 更新 (`+ 0.0` で戻す) だけ)。このモードでは `touched` は立てない (消去で `false` を書くのは無害)。
        let price_dense_now = BIG && !price_by_col_now && price_dense_ratio > 0.0 && last_price_entries as f64 > price_dense_ratio * n_total as f64;
        // この反復の PRICE 要素数。
        let mut n_entries = 0usize;
        timed!(profile_phases, prof_phases::PRICE, {
            // この反復で PRICE した(非ゼロの)行数。
            let mut n_priced = 0usize;
            // 1 行分の PRICE。両ループに展開するためクロージャではなくマクロにしている。
            macro_rules! price_row {
                ($i:expr) => {{
                    let i: usize = $i;
                    let rv = rho[i];
                    if !(rv.abs() <= TOL) {
                        n_priced += 1;
                        let (lo, hi) = (price_start[i], price_nb_end[i]);
                        n_entries += hi - lo;
                        if price_dense_now {
                            // 策3 (密結果モード): 初到達の判定と `touched_cols` への積み込みをせず加算だけ。
                            price_row_dense(&mut a_p, &price_col[lo..hi], &price_val[lo..hi], rv);
                        } else if prefetch_on {
                            price_row_prefetch(&mut a_p, &mut touched, &mut touched_cols, &price_col[lo..hi], &price_val[lo..hi], rv, prefetch_dist);
                        } else {
                            for (&j, &v) in price_col[lo..hi].iter().zip(&price_val[lo..hi]) {
                                let j = j as usize;
                                if !touched[j] {
                                    touched[j] = true;
                                    touched_cols.push(j);
                                }
                                a_p[j] += rv * v;
                            }
                        }
                    }
                }};
            }
            if price_by_col_now {
                n_priced = rho_count_for_col;
            } else if let Some(rows) = if BIG { btran_work.nonzero_rows() } else { None } {
                // 策5: 超疎 BTRAN が返した非ゼロ行(昇順)をそのまま使う(`compact_rows` と同じ行・順序)。
                for (dst, &i) in rho_rows.iter_mut().zip(rows) {
                    *dst = i as u32;
                }
                for &i in rows {
                    price_row!(i);
                }
                rho_list_len = Some(rows.len());
            } else if (last_rho_nnz as f64) <= tunable!("ENOMOTO_T_PRICE_LIST_DENSITY", PRICE_LIST_DENSITY, f64) * m as f64 {
                let k = compact_rows(m, &mut rho_rows, |i| rho[i].to_bits() << 1);
                for &i in &rho_rows[..k] {
                    price_row!(i as usize);
                }
                rho_list_len = Some(k);
            } else {
                for i in 0..m {
                    price_row!(i);
                }
            }
            last_rho_nnz = n_priced;
            if price_dense_now {
                for (j, v) in a_p.iter().enumerate() {
                    if v.to_bits() != 0 {
                        touched_cols.push(j);
                    }
                }
            }
        });
        last_price_entries = n_entries;
        // 診断(`ENOMOTO_PROF_PHASES_EXT_WORK`): PRICE の作業量を集計する。
        if profile_work {
            use std::sync::atomic::Ordering::Relaxed;
            let mut rho_nnz = 0usize;
            let mut entries = 0usize;
            let mut entries_nb = 0usize;
            for i in 0..m {
                if rho[i].abs() > TOL {
                    rho_nnz += 1;
                    entries += price_start[i + 1] - price_start[i];
                    entries_nb += price_col[price_start[i]..price_start[i + 1]].iter().filter(|&&j| nb_status[j as usize].is_some()).count();
                }
            }
            prof_phases::STAT_RHO_NNZ.fetch_add(rho_nnz, Relaxed);
            prof_phases::STAT_PRICE_ENTRIES.fetch_add(entries, Relaxed);
            prof_phases::STAT_PRICE_ENTRIES_NB.fetch_add(entries_nb, Relaxed);
            prof_phases::STAT_TOUCHED.fetch_add(touched_cols.len(), Relaxed);
            prof_phases::STAT_TOUCHED_NONBASIC.fetch_add(touched_cols.iter().filter(|&&j| nb_status[j].is_some()).count(), Relaxed);
        }

        // 作業 #8 対処 6: 再分解直後 (FT 更新 0 回) の段階 B の行 `r` で、LU の FTRAN による `x_B[r]` と BTRAN の `rho` による
        // `rho^T (b - N x_N)` を照合する。両者が相対 `xb_consistency_tol` を超えてずれるなら基底は数値的に壊れている
        // (pilot87 の誤 infeasible では -7.34 と +2.70)ので、最初の求解では摂動を掛け直して安全モードで解き直す。
        if (xb_consistency_tol > 0.0 || noise_diag) && phase == Phase::B && lu.update_count() == 0
            && xb_consistency_bailout(&rho, rho_list_len.map(|k| &rho_rows[..k]), &rhs_inc_base, x_b_base[r], xb_consistency_tol, safe_pivot, noise_diag, iter_idx, r, &mut diag_xb_mismatch)
        {
            return None;
        }
        // Eligible 内の `Zero` 列(あれば BFRT を行わずこれにピボットする)。
        let mut zero_pick: Option<Cand>;
        // この反復で行 `r` を一時的にプールから外すか(`STUCK_ROW_MIN_PIVOT` 参照)。
        let mut stuck_row_taboo = false;
        // 策7: 候補列を `candidates` に写さず `cand_scratch[..n]` のまま使う場合の候補数 `n`。
        let mut fast_n: Option<usize> = None;
        timed!(profile_phases, prof_phases::CHUZC1, {
            candidates.clear();
            // この行 `r` について破棄済み候補の禁止が有効か。
            let ban_active = ban_discarded_candidates && discard_row == Some(r);
            // 分岐なしの候補フィルタ: 触れた列をすべて次の枠に書き込み、3 条件(非基底、
            // `|alpha_j| > TOL`(NaN を同様に扱うため `<=` の否定形)、比率テストの符号条件)を
            // 満たすときだけ `k += keep` で残す。残った候補は `touched_cols` の順。
            // `cand_scratch` は伸びるだけ(縮めない)。
            if cand_scratch.len() < touched_cols.len() {
                cand_scratch.resize(touched_cols.len(), Cand { j: 0, hat_alpha: 0.0, ratio: 0.0 });
            }
            let d_dir_f = d_dir as f64;
            // 残した候補数。
            let mut k = 0usize;
            // 1 列分のフィルタ (本体は [`chuzc1_filter_one`]、プリフェッチ付きの経路と共有)。
            macro_rules! chuzc1_filter_col {
                ($j:expr) => {{
                    let j: usize = $j;
                    let (c, keep) = chuzc1_filter_one(j, a_p[j], nb_status[j], d[j], d_dir_f);
                    cand_scratch[k] = c;
                    k += keep as usize;
                }};
            }
            if prefetch_on {
                k = chuzc1_filter_prefetch(&touched_cols, &a_p, &nb_status, &d, &mut cand_scratch, d_dir_f, prefetch_dist);
            } else {
                for &j in &touched_cols {
                    chuzc1_filter_col!(j);
                }
            }
            // 作業 #5 M1': 行 `r` が数値的に従属に近い(`sqrt(w_r) > NOISE_MIN_SQRT_W`)ときだけ、PRICE 行の雑音水準
            // `C eps sqrt(w_r) ‖a_j‖∞` 以下の候補を外す(逸脱が本物でも、その行の `alpha_j` は丸め誤差のことがある)。
            // 全滅したらこの行を一時的にプールから外す(`stuck_row_taboo` と同じ処理)。
            if noise_on && k > 0 {
                let wr = dse.weight(r);
                if wr > noise_min_w {
                    let k2 = filter_noise_pivots(std, &mut col_inf_norm, &mut cand_scratch[..k], noise_c * f64::EPSILON * wr.sqrt(), noise_dry);
                    if k2 < k {
                        diag_noise_pivot_iters += 1;
                        diag_noise_pivot_cands += k - k2;
                        if !noise_dry {
                            if k2 == 0 {
                                diag_noise_taboo += 1;
                                stuck_row_taboo = true;
                            }
                            k = k2;
                        }
                    }
                }
            }
            // 作業 #5 M3: 巻き戻したピボットの `(r, q)` を外す。全滅したらこの行を一時的にプールから外す。
            if rb_ban_row[r] && k > 0 {
                let k2 = filter_banned_pivots(r, &rb_bans, &mut cand_scratch[..k]);
                if k2 == 0 {
                    stuck_row_taboo = true;
                }
                k = k2;
            }
            // 破棄直後の行の再試行(`STUCK_ROW_MIN_PIVOT` 参照):
            // - 下限を満たす候補が 1 つも無く他に実行不能行があれば、この行を一時的にプールから外して
            //   別の行を選ばせる(HiGHS のタブー行と同じ考え。下の `stuck_row_taboo` 参照)。常に行う。
            // - 安全モード(`safe_pivot`)では、下限を満たす候補があるとき極小ピボットを候補から外す
            //   (順序は保つ)。経路を変える範囲が広いので通常モードでは行わない。
            // 他に行が無ければ何もせず、従来どおり極小ピボットを使う。
            if stuck_row == Some(r) {
                let min_pivot = tunable!("ENOMOTO_T_STUCK_ROW_MIN_PIVOT", STUCK_ROW_MIN_PIVOT, f64);
                let any_large = cand_scratch[..k].iter().any(|c| c.hat_alpha.abs() > min_pivot);
                if any_large && safe_pivot {
                    let mut w = 0usize;
                    for idx in 0..k {
                        if cand_scratch[idx].hat_alpha.abs() > min_pivot {
                            cand_scratch[w] = cand_scratch[idx];
                            w += 1;
                        }
                    }
                    k = w;
                } else if !any_large && infeasible_rows.rows.len() > 1 {
                    stuck_row_taboo = true;
                }
            }
            let kept = &cand_scratch[..k];
            // 論文 3.1 節 (iii)・注意 3.1・Algorithm 1: Eligible に `Zero` 列があれば BFRT を行わず、そのうち
            // 最小添字の列にピボットする(比 0、被約費用もフリップも変化なし)。下の停止候補による
            // 刈り込みより前に `kept` から選ぶ。
            zero_pick = None;
            if n_zero_nonbasic > 0 {
                for c in kept {
                    if nb_status[c.j] == Some(NbStatus::Zero) && zero_pick.map_or(true, |z: Cand| c.j < z.j) && !(ban_active && discard_banned_cols.contains(&c.j)) {
                        zero_pick = Some(*c);
                    }
                }
            }
            // 幅が真の無限(`cache.width[j] == None`、片側行のスラック)の候補のうち `(ratio, j)`
            // 順で最小のもの(停止候補)。BFRT の歩進はそこで無条件に止まるので、それより後ろの
            // 候補は何にも影響しない。それらを捨てても選ばれる `q`・フリップ集合・浮動小数点値は
            // 不変で、ヒープの元になる候補数だけが減る(HiGHS `choosePossible` と同じ考え)。
            // 策7 を検討する反復か (上位候補の選択を使う反復)。
            let fast_eligible = chuzc1_fast_allowed && chuzc1_topk > 0 && !ban_active && !bland_mode && k > chuzc1_topk;
            if fast_eligible {
                chuzc1_fast_probe += 1;
            }
            if fast_eligible && chuzc1_fast_on && chuzc1_fast_probe % CHUZC1_FAST_PROBE != 0 {
                // 策7 (`chuzc1_fast`、幅無限の列が少ない問題のみ): 下の策8 (上位候補の選択) を使う場合に、停止候補
                // による刈り込みと `candidates` への写しを省き、`cand_scratch[..k]` をそのまま候補列にする。歩進は
                // `(ratio, j)` 順で最初の幅無限の候補 (= 停止候補) で必ず止まるので結果は同じ。
                fast_n = Some(k);
            } else {
                let mut stopper: Option<Cand> = None;
                for c in kept {
                    if width_inf[c.j] && stopper.map_or(true, |s| *c < s) && !(ban_active && discard_banned_cols.contains(&c.j)) {
                        stopper = Some(*c);
                    }
                }
                candidates.extend(kept.iter().filter(|c| stopper.map_or(true, |s| **c <= s) && !(ban_active && discard_banned_cols.contains(&c.j))));
                if fast_eligible {
                    // 策7 の測定: 刈り込みで半分より多く残ったら、次の測定まで刈り込みを省く。
                    chuzc1_fast_on = 2 * candidates.len() > k;
                }
            }
        });
        if stuck_row_taboo {
            // 破棄直後の行 `r` に有効な大きさのピボットが無い: 極小ピボットで基底を特異に近づける
            // 代わりに、`r` を一時的に実行不能行プールから外して次の反復で別の行を選ぶ。`x_B[r]` が
            // 変われば `refresh_row` が、再分解では `rebuild_rows` が `r` を戻す(`stuck_row` は残るので、
            // 戻った `r` を選べば同じ判定をやり直す)。プールが空になっても `finish` 後の
            // `polish_with_true_bounds` が真の境界で主実行可能性を検査し直す。
            for &j in &touched_cols {
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            candidates.clear();
            infeasible_rows.set(r, false);
            stuck_row_tabooed[r] = true;
            continue;
        }
        if profile_work {
            prof_phases::STAT_CANDS.fetch_add(candidates.len() + fast_n.unwrap_or(0), std::sync::atomic::Ordering::Relaxed);
        }
        if candidates.is_empty() && fast_n.is_none() {
            for &j in &touched_cols {
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            // Eligible が空。`super::solve_lp_dual_on` と同じ noise_feasible の再挑戦: 行 `r` の
            // 逸脱が丸め誤差程度(`bfrt_reached` の尺度付き許容誤差)なら、以後この基底変数を
            // 実行可能とみなして除外する。傾きが非ゼロの逸脱は `M` のオーダーなので対象外。
            if bfrt_reached(w_r, Affine1::ZERO, x_b_base[r]) {
                noise_feasible[basis[r]] = true;
                row_bounds.mark_noise(r);
                infeasible_rows.set(r, false);
                continue;
            }
            // 更新済み(`update_count > 0`)の分解からは実行不能と結論しない: FT 更新の累積誤差だけで
            // 実行可能な行が違反に見えることがあるので、新しい分解で反復をやり直し、そこでも
            // 成り立つときだけ `Infeasible` を報告する。再分解で `update_count() == 0` になるので
            // 必ず停止する。
            if lu.update_count() > 0 {
                if profile_phases {
                    prof_phases::REFACTOR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    prof_phases::REFACTOR_CAUSE_INFEAS_CHECK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                timed!(profile_phases, prof_phases::REFACTOR, {
                    lu = refactor_main!();
                    let (fresh_base, fresh_slope) = resync_x_b(std, &cache, &nb_status, &lu, phase, &mut lu_scratch, &mut x_b_base, &mut x_b_slope)?;
                    rhs_inc_base = fresh_base;
                    rhs_inc_slope = fresh_slope;
                    rebuild_rows(&mut infeasible_rows, &mut row_dev, m, &row_bounds, &x_b_base, &x_b_slope);
                    fresh_d_into(std, &lu, &basis, &basis_pos, &active_cost, &mut fresh_d_cb, &mut lu_scratch, &mut fresh_d_y, &mut d);
                    if dse_refresh_on_refactor {
                        dse = super::DseState::from_basis(m, &lu);
                    }
                });
                continue;
            }
            // 作業 #5 M2: 行が雑音水準か数値的に従属なら結論せず、行を一時的に外して続ける(プールが空になれば
            // `finish` 後の `polish_with_true_bounds` が真の境界で確かめ直す)。
            infeas_check!("eligible_empty", iter_idx, r, w_r);
            // 真の数学的結論(論文 5.4 節 Step 2(c)・命題 7.4 の `Eligible = ∅` による実行不能判定)。
            // 診断(`ENOMOTO_DEBUG_EXT_INFEASIBLE`): 行情報と双対実行可能性違反の数を出力する。
            if env_str!("ENOMOTO_DEBUG_EXT_INFEASIBLE").is_some() {
                eprintln!(
                    "DEBUG_EXT_INFEASIBLE: site=eligible_empty iter={iter_idx} r={r} basis_r={} d_dir={d_dir} w_r=({},{}) noise_feasible_check_failed=true remaining_m_side={remaining_m_side}",
                    basis[r], w_r.base, w_r.slope
                );
                let mut dual_violations = 0usize;
                for j in 0..std.n_total {
                    let Some(status) = nb_status[j] else { continue };
                    let dj = d[j];
                    let bad = match status {
                        NbStatus::Lower => dj < -1e-6,
                        NbStatus::Upper => dj > 1e-6,
                        NbStatus::Zero => dj.abs() > 1e-6,
                    };
                    if bad {
                        dual_violations += 1;
                        if dual_violations <= 5 {
                            eprintln!("  DUAL_FEAS_VIOLATION: j={j} status={status:?} d[j]={dj} delta[j]={:?}", if j < n_orig { delta[j] } else { MSide::None });
                        }
                    }
                }
                eprintln!("  total dual_feasibility_violations={dual_violations} (out of nonbasic columns)");
            }
            if profile_phases {
                prof_phases::report(wall_t0.elapsed().as_nanos() as usize);
            }
            if phase == Phase::A {
                // 傾き問題は `x^1 = 0` で実行可能(論文 7.1 節)なので段階 A が実行不能を証明することは
                // 無い。ここに来たのは数値的破綻なので `None`(`NotSolved`)で抜ける。
                if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
                    eprintln!("DEBUG_EXT_BAILOUT: stage A found no entering column at iter={iter_idx} r={r} (numerical)");
                }
                return None;
            }
            return Some(SimplexResult { status: Status::Infeasible, x: None });
        }
        // chuzc1 + BFRT パス 1: `bland_mode` では候補全体を並べる必要があるが、通常は歩進が
        // 先頭の 1〜2 候補で止まるので、`(ratio, j)` 昇順の最小ヒープ
        // (`BinaryHeap<Reverse<Cand>>`、構築 `O(k)`)から 1 つずつ取り出す。
        // `candidates.drain(..)` で割り当てを次反復に残す。
        // `n_candidates`: 候補数(診断出力用)。`sorted_prefix`: ヒープから取り出した先頭部分。
        // `k_star`: 累積フリップ容量が `w_r` に達した候補の位置(達しなければ `None`)。
        let n_candidates = candidates.len() + fast_n.unwrap_or(0);
        sorted_prefix.clear();
        let mut k_star: Option<usize> = None;
        // BFRT の `M` 付きへの一般化: 累積フリップ容量 `cum` は `Affine1` の累積和で、`w_r` と辞書式に
        // 比較する。幅が真の無限 (`None`) の候補に来たら即座に止まる。`k_star == None` の
        // 診断出力でも読むので分岐の外で宣言する。
        let mut cum = Affine1::ZERO;
        if let Some(zc) = zero_pick {
            candidates.clear();
            sorted_prefix.push(zc);
            k_star = Some(0);
        } else if bland_mode {
            timed!(profile_phases, prof_phases::CHUZC1, {
                // 非 `bland` のヒープ経路と同じ `(ratio, j)` 昇順(`j` だけの順ではない)。
                // BFRT の歩進は比の昇順を前提とし、`j` は同値時の決定的なタイブレーク。
                candidates.sort();
            });
            timed!(profile_phases, prof_phases::BFRT, {
                for (idx, cand) in candidates.iter().enumerate() {
                    let Some(width) = cache.width[cand.j] else {
                        k_star = Some(idx);
                        break;
                    };
                    let new_cum = cum.add(width.scale(cand.hat_alpha.abs()));
                    if bfrt_reached(w_r, new_cum, x_b_base[r]) {
                        k_star = Some(idx);
                        break;
                    }
                    cum = new_cum;
                }
            });
        } else if BIG && chuzc1_topk > 0 && (fast_n.is_some() || candidates.len() >= chuzc1_topk_min_cands) {
            // 策8 (`BIG` のみ): 候補全体をヒープ化する代わりに、1 パスで `(ratio, j)` 順の小さい方から
            // `chuzc1_topk` 個を上限付きの最大ヒープに集めて並べ、先に歩進する。BFRT の歩進は
            // 通常フリップ数 + 1 個 (数個〜十数個) で止まるので、数千候補の heapify を省ける。
            // 歩進が止まらなければ、直前の組の最大より大きい候補から次の組 (個数は 4 倍ずつ) を同じように
            // 選んで続ける。取り出す順序は全体ヒープと同じ `(ratio, j)` 昇順なのでビット一致。
            // 候補列 (策7 なら刈り込み前の `cand_scratch[..n]`)。
            let cand_src: &[Cand] = match fast_n {
                Some(n) => &cand_scratch[..n],
                None => &candidates[..],
            };
            // 候補 1 つ分の歩進 (下の全体ヒープ経路と同じ)。止まったら `true`。
            macro_rules! bfrt_step {
                ($cand:expr) => {{
                    let cand: Cand = $cand;
                    let idx = sorted_prefix.len();
                    sorted_prefix.push(cand);
                    match cache.width[cand.j] {
                        None => {
                            k_star = Some(idx);
                            true
                        }
                        Some(width) => {
                            let new_cum = cum.add(width.scale(cand.hat_alpha.abs()));
                            if bfrt_reached(w_r, new_cum, x_b_base[r]) {
                                k_star = Some(idx);
                                true
                            } else {
                                cum = new_cum;
                                false
                            }
                        }
                    }
                }};
            }
            // この組で選ぶ個数と、前の組の最大 (これより大きい候補だけから選ぶ)。
            let mut group = chuzc1_topk;
            let mut after: Option<Cand> = None;
            loop {
                let heap_t0 = profile_phases.then(std::time::Instant::now);
                timed!(profile_phases, prof_phases::CHUZC1, {
                    topk_heap.clear();
                    for c in cand_src.iter() {
                        if after.is_some_and(|a| *c <= a) {
                            continue;
                        }
                        if topk_heap.len() < group {
                            topk_heap.push(*c);
                        } else if let Some(mut top) = topk_heap.peek_mut() {
                            if *c < *top {
                                *top = *c;
                            }
                        }
                    }
                    topk_sorted.clear();
                    topk_sorted.extend(topk_heap.drain());
                    topk_sorted.sort_unstable();
                });
                if let Some(t0) = heap_t0 {
                    prof_phases::CHUZC1_HEAP.fetch_add(t0.elapsed().as_nanos() as usize, std::sync::atomic::Ordering::Relaxed);
                }
                let mut stopped = false;
                timed!(profile_phases, prof_phases::BFRT, {
                    for i in 0..topk_sorted.len() {
                        if bfrt_step!(topk_sorted[i]) {
                            stopped = true;
                            break;
                        }
                    }
                });
                // 止まった、または候補を使い切った (この組が満杯でない)。
                if stopped || topk_sorted.len() < group {
                    break;
                }
                after = topk_sorted.last().copied();
                group = group.saturating_mul(4);
            }
            candidates.clear();
        } else {
            let heap_t0 = profile_phases.then(std::time::Instant::now);
            let mut heap: BinaryHeap<Reverse<Cand>> = timed!(profile_phases, prof_phases::CHUZC1, {
                heap_buf.clear();
                heap_buf.extend(candidates.drain(..).map(Reverse));
                BinaryHeap::from(std::mem::take(&mut heap_buf))
            });
            if let Some(t0) = heap_t0 {
                prof_phases::CHUZC1_HEAP.fetch_add(t0.elapsed().as_nanos() as usize, std::sync::atomic::Ordering::Relaxed);
            }
            timed!(profile_phases, prof_phases::BFRT, {
                while let Some(Reverse(cand)) = heap.pop() {
                    let idx = sorted_prefix.len();
                    sorted_prefix.push(cand);
                    let Some(width) = cache.width[cand.j] else {
                        k_star = Some(idx);
                        break;
                    };
                    let new_cum = cum.add(width.scale(cand.hat_alpha.abs()));
                    if bfrt_reached(w_r, new_cum, x_b_base[r]) {
                        k_star = Some(idx);
                        break;
                    }
                    cum = new_cum;
                }
            });
            heap_buf = heap.into_vec();
        }
        // chuzc1 が作った並び順の候補列: `bland_mode` なら全体を並べた `candidates`、
        // そうでなければ遅延構築した `sorted_prefix`(`[0, k_star]` のみ。パス 2 と
        // フリップはそこしか参照しない)。
        let sorted: &[Cand] = if bland_mode && zero_pick.is_none() { &candidates[..] } else { &sorted_prefix[..] };
        let Some(k_star) = k_star else {
            for &j in &touched_cols {
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            // 上の `Eligible = empty` と同じガード: 更新済みの分解からは実行不能と結論せず、
            // 新しい分解で反復をやり直す。
            if lu.update_count() > 0 {
                if profile_phases {
                    prof_phases::REFACTOR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    prof_phases::REFACTOR_CAUSE_INFEAS_CHECK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                timed!(profile_phases, prof_phases::REFACTOR, {
                    lu = refactor_main!();
                    let (fresh_base, fresh_slope) = resync_x_b(std, &cache, &nb_status, &lu, phase, &mut lu_scratch, &mut x_b_base, &mut x_b_slope)?;
                    rhs_inc_base = fresh_base;
                    rhs_inc_slope = fresh_slope;
                    rebuild_rows(&mut infeasible_rows, &mut row_dev, m, &row_bounds, &x_b_base, &x_b_slope);
                    fresh_d_into(std, &lu, &basis, &basis_pos, &active_cost, &mut fresh_d_cb, &mut lu_scratch, &mut fresh_d_y, &mut d);
                    if dse_refresh_on_refactor {
                        dse = super::DseState::from_basis(m, &lu);
                    }
                });
                continue;
            }
            // 作業 #5 M2(上の `Eligible = ∅` と同じガード)。
            infeas_check!("bfrt_exhausted", iter_idx, r, w_r);
            // 全候補をフリップしてもまだ足りない: 論文 5.4 節 Step 2(c) の実行不能判定の BFRT 版で、真の実行不能。
            if env_str!("ENOMOTO_DEBUG_EXT_INFEASIBLE").is_some() {
                eprintln!(
                    "DEBUG_EXT_INFEASIBLE: site=bfrt_exhausted iter={iter_idx} r={r} basis_r={} d_dir={d_dir} w_r=({},{}) n_candidates={} cum=({},{}) remaining_m_side={remaining_m_side}",
                    basis[r], w_r.base, w_r.slope, n_candidates, cum.base, cum.slope
                );
            }
            if profile_phases {
                prof_phases::report(wall_t0.elapsed().as_nanos() as usize);
            }
            if phase == Phase::A {
                // 段階 A は実行不能を証明できない(傾き問題は `x^1 = 0` で実行可能、論文 7.1 節)ので数値的破綻として `None`。
                if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
                    eprintln!("DEBUG_EXT_BAILOUT: stage A found no entering column at iter={iter_idx} r={r} (numerical)");
                }
                return None;
            }
            return Some(SimplexResult { status: Status::Infeasible, x: None });
        };

        // Harris 型パス 2(`HARRIS_RATIO_TOL` 参照): `k_star` から後ろ向きにだけ(前向きは不可)、
        // 比が平坦な窓の中で |ピボット| 最大の候補を探してそれでピボットする。それより前の
        // 候補はフリップする(パス 1 が `k_star` までのフリップで `w_r` を超えないことを
        // 示しているので、より短い接頭辞も超えない)。`best_idx`: 実際にピボットする候補の位置。
        let mut best_idx = k_star;
        timed!(profile_phases, prof_phases::BFRT, {
            let min_ratio = sorted[k_star].ratio - tunable!("ENOMOTO_T_HARRIS_RATIO_TOL", HARRIS_RATIO_TOL, f64);
            let mut window_start = k_star;
            while window_start > 0 && sorted[window_start - 1].ratio >= min_ratio {
                window_start -= 1;
            }
            let mut best_abs = sorted[k_star].hat_alpha.abs();
            for (idx, cand) in sorted.iter().enumerate().take(k_star + 1).skip(window_start) {
                let abs_a = cand.hat_alpha.abs();
                if abs_a > best_abs {
                    best_abs = abs_a;
                    best_idx = idx;
                }
            }
            // 診断: 窓の大きさと、窓内に(選ばれなかった)M 側候補があったかを数える。
            if profile_phases {
                prof_phases::HARRIS_WINDOW_SIZE_SUM.fetch_add(k_star - window_start + 1, std::sync::atomic::Ordering::Relaxed);
                let window_has_m_elsewhere = sorted[window_start..=k_star].iter().enumerate().any(|(off, cand)| {
                    let idx = window_start + off;
                    if idx == best_idx || !delta[cand.j].is_flagged() {
                        return false;
                    }
                    match nb_status[cand.j] {
                        Some(NbStatus::Lower) => delta[cand.j].has_lower(),
                        Some(NbStatus::Upper) => delta[cand.j].has_upper(),
                        Some(NbStatus::Zero) | None => false,
                    }
                });
                if window_has_m_elsewhere {
                    prof_phases::HARRIS_WINDOW_M_MISS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
        });

        // BFRT 結合フリップ: フリップする全候補の境界変化量(符号付きの `width_affine`)を
        // `Affine1` のチャネルごとに 1 本の疎右辺へ集約し、各チャネル 1 回の FTRAN で解く
        // (密/疎は `should_use_dense_solve` で切り替え)。反転前の `nb_status` を読むので
        // 反転確定ループより前に実行する。`x_b_base`/`x_b_slope` への反映は入る列の FTRAN の後
        // (`combined_pending`)で、既定では入る列の `x_B` 更新ループの中で行う
        // (`merge_flip_xb`)。`theta` は行 `r` のフリップ後の値から計算する。
        if profile_phases {
            prof_phases::BFRT_FLIPS.fetch_add(best_idx, std::sync::atomic::Ordering::Relaxed);
        }
        // `combined_pending`: 結合フリップの FTRAN 結果(`combined_deferred` のときは未求解の
        // 右辺)が `x_B(M)` への反映待ちか。反映は入る列の FTRAN(`x_B(M)` も
        // `InfeasibleRows` も読まない)の直後に行うので、遅らせても経路は変わらない。
        let mut combined_pending = false;
        // 基底チャネルの求解を入る列の融合 FTRAN に相乗りさせるか。
        let mut combined_deferred = false;
        // 傾きチャネルに非ゼロ(M 項を持つフリップ)があったか。
        let mut combined_slope_nonzero = false;
        // 傾きチャネルの FTRAN 結果の非ゼロ数(遅延時に密度記録で使う)。
        let mut combined_slope_nnz = 0usize;
        // 結合フリップ結果の非ゼロ数の上界(両チャネルの和)。`x_B` 更新の一覧/全走査の選択用。
        let mut combined_nnz = 0usize;
        timed!(profile_phases, prof_phases::BFRT, {
            // フリップ列の幅に `M` 項がある(`width.slope != 0`)か。無ければ `combined_slope` は
            // すべて正確な(符号付き)0 なので傾きチャネルの FTRAN を省略し、その合成クロックの
            // tick とゼロ密度サンプルだけを再現する(CLOCK トリガと密/疎切り替え、ひいては
            // ピボット経路はビット単位で不変)。
            let mut slope_nonzero = false;
            for cand in &sorted[..best_idx] {
                let old = nb_status[cand.j].unwrap();
                let width = cache.width[cand.j].unwrap();
                debug_assert_ne!(old, NbStatus::Zero, "a `Zero` column is never flipped");
                let sigma = if old == NbStatus::Lower { 1.0 } else { -1.0 };
                let delta_x = width.scale(sigma);
                if delta_x.slope != 0.0 {
                    slope_nonzero = true;
                    for &(i, v) in std.cols.col(cand.j) {
                        if !combined_touched_flag[i] {
                            combined_touched_flag[i] = true;
                            combined_touched.push(i);
                        }
                        combined_base[i] += v * delta_x.base;
                        combined_slope[i] += v * delta_x.slope;
                    }
                } else if phase != Phase::A {
                    // (段階 A: 傾きを持たないフリップ(箱型列、`s_j = 0`)は段階 A が追跡しない
                    // 切片 `x^0` しか動かさないので、解くべき寄与は無い。)
                    for &(i, v) in std.cols.col(cand.j) {
                        if !combined_touched_flag[i] {
                            combined_touched_flag[i] = true;
                            combined_touched.push(i);
                        }
                        combined_base[i] += v * delta_x.base;
                    }
                }
            }
            if !combined_touched.is_empty() {
                #[cfg(test)]
                COMBINED_FLIP_COUNT.fetch_add(best_idx, std::sync::atomic::Ordering::Relaxed);
                // 両チャネルは同じ右辺パターンを同じ基底で解くので密度履歴を共有する。
                if profile_phases && density_bfrt.predicts_dense() && !lu.should_use_dense_solve(combined_touched.len()) {
                    prof_phases::DENSITY_GATE_FTRANS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                if phase == Phase::A {
                    // 段階 A: 傾きチャネルのみ(基底チャネルは恒等的に 0 なので解かず、
                    // 入る列の FTRAN にも融合しない)。`combined_alpha_base` は段階 A では
                    // 書かれず 0 のまま。
                    debug_assert!(slope_nonzero);
                    let slope_nnz = if lu.should_use_dense_solve_tracked(combined_touched.len(), &density_bfrt) {
                        lu.solve_into(&combined_slope, &mut lu_scratch, &mut combined_alpha_slope)
                    } else {
                        sparse_slope_buf.clear();
                        sparse_slope_buf.extend(combined_touched.iter().map(|&i| (i, combined_slope[i])));
                        lu.solve_sparse_into(&sparse_slope_buf, &mut sparse_scratch, &mut gp_scratch, &mut combined_alpha_slope)
                    };
                    cas_track.set_full();
                    density_bfrt.record(slope_nnz, m);
                    combined_nnz = slope_nnz;
                } else if lu.should_use_dense_solve_tracked(combined_touched.len(), &density_bfrt) {
                    // 傾きチャネル(まれ)はここで単独に解く。基底チャネルは入る列の融合 FTRAN に
                    // 相乗りする (`combined_deferred`) か、ここで解く。密度サンプルはどちらでも
                    // (基底, 傾き) の順に記録する。
                    let slope_nnz = if slope_nonzero {
                        cas_track.set_full();
                        lu.solve_into(&combined_slope, &mut lu_scratch, &mut combined_alpha_slope)
                    } else {
                        lu.add_zero_rhs_solve_ticks(false);
                        0
                    };
                    // 基底チャネルはこの後の融合 FTRAN かここで全体を書く。
                    cab_track.set_full();
                    if fused_bfrt_ftran && fused_dse_ftran {
                        combined_deferred = true;
                        combined_slope_nnz = slope_nnz;
                    } else {
                        let base_nnz = lu.solve_into(&combined_base, &mut lu_scratch, &mut combined_alpha_base);
                        density_bfrt.record(base_nnz, m);
                        density_bfrt.record(slope_nnz, m);
                        combined_nnz = base_nnz + slope_nnz;
                        if profile_phases {
                            prof_phases::DENSITY_BFRT_PPT.store((density_bfrt.expected() * 1000.0) as usize, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                } else {
                    sparse_base_buf.clear();
                    sparse_slope_buf.clear();
                    sparse_base_buf.extend(combined_touched.iter().map(|&i| (i, combined_base[i])));
                    // 結果密度が低ければ `U` 段を超疎で解く(入る列の FTRAN と同じ `FTRAN_U_HYPER_DENSITY`、ビット同一)。
                    // (大きな問題のみ。`solve_sparse_into_hyper`)
                    let u_hyper_gate = tunable!("ENOMOTO_FTRAN_U_HYPER", FTRAN_U_HYPER_DENSITY, f64);
                    gp_scratch.u_hyper = BIG && u_hyper_gate > 0.0 && density_bfrt.expected() < u_hyper_gate;
                    let base_nnz = if flip_solve_track {
                        lu.solve_sparse_into_hyper_tracked(&sparse_base_buf, &mut sparse_scratch, &mut gp_scratch, &mut combined_alpha_base, &mut cab_track)
                    } else if BIG {
                        cab_track.set_full();
                        lu.solve_sparse_into_hyper(&sparse_base_buf, &mut sparse_scratch, &mut gp_scratch, &mut combined_alpha_base)
                    } else {
                        lu.solve_sparse_into(&sparse_base_buf, &mut sparse_scratch, &mut gp_scratch, &mut combined_alpha_base)
                    };
                    let slope_nnz = if slope_nonzero {
                        sparse_slope_buf.extend(combined_touched.iter().map(|&i| (i, combined_slope[i])));
                        if flip_solve_track {
                            lu.solve_sparse_into_hyper_tracked(&sparse_slope_buf, &mut sparse_scratch, &mut gp_scratch, &mut combined_alpha_slope, &mut cas_track)
                        } else if BIG {
                            cas_track.set_full();
                            lu.solve_sparse_into_hyper(&sparse_slope_buf, &mut sparse_scratch, &mut gp_scratch, &mut combined_alpha_slope)
                        } else {
                            lu.solve_sparse_into(&sparse_slope_buf, &mut sparse_scratch, &mut gp_scratch, &mut combined_alpha_slope)
                        }
                    } else {
                        lu.add_zero_rhs_solve_ticks(true);
                        0
                    };
                    gp_scratch.u_hyper = false;
                    density_bfrt.record(base_nnz, m);
                    density_bfrt.record(slope_nnz, m);
                    combined_nnz = base_nnz + slope_nnz;
                    if profile_phases {
                        prof_phases::DENSITY_BFRT_PPT.store((density_bfrt.expected() * 1000.0) as usize, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                combined_pending = true;
                combined_slope_nonzero = slope_nonzero;
            }

            for cand in &sorted[..best_idx] {
                let old = nb_status[cand.j].unwrap();
                let new = match old {
                    NbStatus::Lower => NbStatus::Upper,
                    NbStatus::Upper => NbStatus::Lower,
                    NbStatus::Zero => unreachable!("a `Zero` column is never flipped"),
                };
                if profile_phases && delta[cand.j].is_flagged() {
                    let lands_on_m = match new {
                        NbStatus::Lower => delta[cand.j].has_lower(),
                        NbStatus::Upper => delta[cand.j].has_upper(),
                        NbStatus::Zero => false,
                    };
                    if lands_on_m {
                        prof_phases::M_ENTER_VIA_FLIP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                nb_status[cand.j] = Some(new);
                pivot_flips.push(cand.j);
            }
        });
        // 入る列 `q`、その被約費用 `dj_q`、PRICE によるピボット要素 `alpha_q`。
        let q = sorted[best_idx].j;
        let dj_q = d[q];
        let alpha_q = a_p[q];
        if profile_phases && dj_q.abs() <= 1e-9 {
            prof_phases::DEGENERATE_PIVOTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if debug_dual_check {
            prev_q = Some(q);
            prev_r = Some(r);
            prev_alpha_q = alpha_q;
            prev_dj_q = dj_q;
            prev_pivot_debug = Some((a_p.clone(), sorted[..best_idx].iter().map(|c| c.j).collect(), sorted.len()));
        }

        // FTRAN: `alpha_full = B^-1 A_q`(ピボット前の `lu` に対して)。DSE 重み更新、
        // `x_B(M)` の増分更新、下の updateVerify で使う。`alpha_q` と `alpha_full[r]` は
        // 同じピボット要素を別経路(BTRAN+内積 と FTRAN)で求めたもの。
        // 列の非ゼロ数と結果密度履歴で密/疎ソルブを切り替える。密分岐では生の列を
        // `dense_q` に散布して右辺とする。どちらの分岐も `try_update_precomputed` 用に
        // `a_tilde_buf`(L/R 後・U 前の中間値)をキャプチャする。
        // DSE の `tau = B^-1 rho` が融合 FTRAN で求まったか。
        let mut tau_ready = false;
        // 遅延した結合フリップ基底チャネルの FTRAN 結果の非ゼロ数。
        let mut combined_base_nnz = 0usize;
        // `alpha_full` の非ゼロ数(FTRAN 自身が数えた値)。
        let mut alpha_nnz = m;
        // 融合 FTRAN が `tau` を求めたときの結果非ゼロ数。
        let mut tau_nnz: Option<usize> = None;
        // C5: `tau` の結果密度がこの値未満なら `U` 段を超疎で解く(`ENOMOTO_FTRAN_U_HYPER_TAU`、0 でオフ)。
        let u_hyper_tau_gate = tunable!("ENOMOTO_FTRAN_U_HYPER_TAU", FTRAN_U_HYPER_TAU_DENSITY, f64);
        rho_steps.set_u_hyper(u_hyper_tau_gate > 0.0 && density_tau.expected() < u_hyper_tau_gate);
        // 部分 `tau` は DSE 重み更新が入る列の非ゼロ行だけを読む場合(フリップ結果の併合なし)に限る。
        ftran_track.partial_tau = partial_tau && !combined_pending;
        timed!(profile_phases, prof_phases::FTRAN, {
            if profile_phases && density_col_aq.predicts_dense() && !lu.should_use_dense_solve(std.cols.col(q).len()) {
                prof_phases::DENSITY_GATE_FTRANS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            if lu.should_use_dense_solve_tracked(std.cols.col(q).len(), &density_col_aq) {
                // `dense_q` は反復間で常に 0 に保ち、密分岐でだけ散布→求解→同じパターンを
                // 0 に戻す(毎回 `O(m)` の `fill` をしない)。
                for &(i, v) in std.cols.col(q) {
                    dense_q[i] = v;
                }
                let result_nnz = if combined_deferred {
                    let (a_nnz, b_nnz, c_nnz) = if BIG { lu.solve_into_triple_capture_tracked(
                        &dense_q,
                        &rho,
                        &combined_base,
                        &mut lu_scratch,
                        &mut tau_scratch,
                        &mut combined_scratch,
                        &mut alpha_full,
                        &mut tau,
                        &mut combined_alpha_base,
                        &mut a_tilde_buf,
                        Some(&mut rho_steps),
                        sparse_ftran_out.then_some(&mut ftran_track),
                        Some(std.cols.col(q)),
                    ) } else { lu.solve_into_triple_capture(
                        &dense_q,
                        &rho,
                        &combined_base,
                        &mut lu_scratch,
                        &mut tau_scratch,
                        &mut combined_scratch,
                        &mut alpha_full,
                        &mut tau,
                        &mut combined_alpha_base,
                        &mut a_tilde_buf,
                        Some(&mut rho_steps),
                    ) };
                    combined_base_nnz = c_nnz;
                    tau_ready = true;
                    tau_nnz = Some(b_nnz);
                    a_nnz
                } else if fused_dse_ftran {
                    // DSE の `tau = B^-1 rho_p` FTRAN を同じ走査に融合する
                    // (`solve_into_pair_capture` 参照)。
                    let (a_nnz, b_nnz) = if BIG {
                        lu.solve_into_pair_capture_tracked(&dense_q, &rho, &mut lu_scratch, &mut tau_scratch, &mut alpha_full, &mut tau, &mut a_tilde_buf, Some(&mut rho_steps), sparse_ftran_out.then_some(&mut ftran_track), Some(std.cols.col(q)))
                    } else {
                        lu.solve_into_pair_capture(&dense_q, &rho, &mut lu_scratch, &mut tau_scratch, &mut alpha_full, &mut tau, &mut a_tilde_buf, Some(&mut rho_steps))
                    };
                    tau_ready = true;
                    tau_nnz = Some(b_nnz);
                    a_nnz
                } else {
                    if BIG {
                        ftran_track.alpha.set_full();
                        ftran_track.a_tilde.set_full();
                    }
                    lu.solve_into_capture(&dense_q, &mut lu_scratch, &mut alpha_full, &mut a_tilde_buf)
                };
                for &(i, _) in std.cols.col(q) {
                    dense_q[i] = 0.0;
                }
                alpha_nnz = result_nnz;
                density_col_aq.record(result_nnz, m);
            } else {
                // C5 (`ENOMOTO_FTRAN_U_HYPER=<gate>`、既定 0.1、0 でオフ): 入る列の結果密度が
                // `gate` 未満の間は `U` 段を超疎で解く(ビット同一)。
                let u_hyper_gate = tunable!("ENOMOTO_FTRAN_U_HYPER", FTRAN_U_HYPER_DENSITY, f64);
                gp_scratch.u_hyper = u_hyper_gate > 0.0 && density_col_aq.expected() < u_hyper_gate;
                let result_nnz = if combined_deferred {
                    let (a_nnz, b_nnz, c_nnz) = if BIG { lu.solve_sparse_into_triple_capture_tracked(
                        std.cols.col(q),
                        &rho,
                        &combined_base,
                        &mut sparse_scratch,
                        &mut gp_scratch,
                        &mut tau_scratch,
                        &mut combined_scratch,
                        &mut alpha_full,
                        &mut tau,
                        &mut combined_alpha_base,
                        &mut a_tilde_buf,
                        Some(&mut rho_steps),
                        sparse_ftran_out.then_some(&mut ftran_track),
                    ) } else { lu.solve_sparse_into_triple_capture(
                        std.cols.col(q),
                        &rho,
                        &combined_base,
                        &mut sparse_scratch,
                        &mut gp_scratch,
                        &mut tau_scratch,
                        &mut combined_scratch,
                        &mut alpha_full,
                        &mut tau,
                        &mut combined_alpha_base,
                        &mut a_tilde_buf,
                        Some(&mut rho_steps),
                    ) };
                    combined_base_nnz = c_nnz;
                    tau_ready = true;
                    tau_nnz = Some(b_nnz);
                    a_nnz
                } else if fused_dse_ftran {
                    let (a_nnz, b_nnz) = if BIG { lu.solve_sparse_into_pair_capture_tracked(
                        std.cols.col(q),
                        &rho,
                        &mut sparse_scratch,
                        &mut gp_scratch,
                        &mut tau_scratch,
                        &mut alpha_full,
                        &mut tau,
                        &mut a_tilde_buf,
                        Some(&mut rho_steps),
                        sparse_ftran_out.then_some(&mut ftran_track),
                    ) } else { lu.solve_sparse_into_pair_capture(
                        std.cols.col(q),
                        &rho,
                        &mut sparse_scratch,
                        &mut gp_scratch,
                        &mut tau_scratch,
                        &mut alpha_full,
                        &mut tau,
                        &mut a_tilde_buf,
                        Some(&mut rho_steps),
                    ) };
                    tau_ready = true;
                    tau_nnz = Some(b_nnz);
                    a_nnz
                } else {
                    if BIG {
                        ftran_track.alpha.set_full();
                        ftran_track.a_tilde.set_full();
                    }
                    lu.solve_sparse_into_capture(std.cols.col(q), &mut sparse_scratch, &mut gp_scratch, &mut alpha_full, &mut a_tilde_buf)
                };
                gp_scratch.u_hyper = false;
                alpha_nnz = result_nnz;
                density_col_aq.record(result_nnz, m);
            }
            if let Some(n) = tau_nnz {
                // 作業 #10 (B): 部分 `tau` の非ゼロ数は入る列の非ゼロ行の分だけで `tau` の密度ではないので、
                // 密度の移動平均 (超疎 `U` 段の選択と合成クロックが読む) には入れない。
                if !(BIG && partial_tau_skip_density && ftran_track.tau_is_partial()) {
                    density_tau.record(n, m);
                }
            }
            if profile_phases {
                prof_phases::DENSITY_COL_AQ_PPT.store((density_col_aq.expected() * 1000.0) as usize, std::sync::atomic::Ordering::Relaxed);
            }
        });
        if combined_deferred {
            combined_nnz = combined_base_nnz + combined_slope_nnz;
            density_bfrt.record(combined_base_nnz, m);
            density_bfrt.record(combined_slope_nnz, m);
            if profile_phases {
                prof_phases::DENSITY_BFRT_PPT.store((density_bfrt.expected() * 1000.0) as usize, std::sync::atomic::Ordering::Relaxed);
            }
        }
        if combined_pending && !merge_flip_xb {
            combined_pending = false;
            timed!(profile_phases, prof_phases::BFRT, {
                // FTRAN のフィルインで入力パターン外にも非ゼロが出るので `0..m` を走査する。
                if combined_slope_nonzero {
                    for i in 0..m {
                        if combined_alpha_base[i] != 0.0 || combined_alpha_slope[i] != 0.0 {
                            x_b_base[i] -= combined_alpha_base[i];
                            x_b_slope[i] = snap_slope(x_b_slope[i] - combined_alpha_slope[i]);
                            refresh_row(&mut infeasible_rows, &mut row_dev, &row_bounds, &x_b_base, &x_b_slope, i);
                        }
                    }
                    for &i in &combined_touched {
                        rhs_inc_base[i] -= combined_base[i];
                        // 空の `rhs_inc_slope` は `delta = 0` 表現。非ゼロの傾き寄与が来たときだけ実体化する。
                        if rhs_inc_slope.is_empty() && combined_slope[i] != 0.0 {
                            rhs_inc_slope.resize(m, 0.0);
                        }
                        if !rhs_inc_slope.is_empty() {
                            rhs_inc_slope[i] -= combined_slope[i];
                        }
                        combined_base[i] = 0.0;
                        combined_slope[i] = 0.0;
                        combined_touched_flag[i] = false;
                    }
                } else {
                    // 傾きチャネルは省略済み(`slope_nonzero` 参照): その結果はすべて `±0.0` で、
                    // `!= 0.0` 判定にも `snap_slope(x - ±0.0) == x` にも影響しない。
                    for i in 0..m {
                        if combined_alpha_base[i] != 0.0 {
                            x_b_base[i] -= combined_alpha_base[i];
                            refresh_row(&mut infeasible_rows, &mut row_dev, &row_bounds, &x_b_base, &x_b_slope, i);
                        }
                    }
                    for &i in &combined_touched {
                        rhs_inc_base[i] -= combined_base[i];
                        combined_base[i] = 0.0;
                        combined_touched_flag[i] = false;
                    }
                }
                combined_touched.clear();
            });
        }

        // updateVerify(HiGHS `HEkkDualRow::updateVerify` 相当): ピボット要素を PRICE の値
        // (`alpha_q`)と FTRAN の値(`alpha_full[r]`)で照合する。`alpha_full` 由来の変更を
        // 確定する前に置く。失敗したらこのピボットを破棄し、再分解・`x_B(M)`/`InfeasibleRows`/
        // `d` の再同期をして次の反復で chuzr/chuzc をやり直す。この反復ですでに確定した
        // BFRT フリップは戻さない(独立した有効な退化ステップで、再同期がフリップ後の
        // `nb_status` から `x_B(M)` を作り直す)。
        //
        // 厳しい `pivot_values_agree` は `lu.update_count() > 0` のときだけ使う(再分解直後に
        // 拒否しても同じ選択が繰り返されるだけのため)。
        //
        // `pivot_grossly_inconsistent` は `update_count` によらず常に行う、ずっと緩い第 2 の
        // チェック: 2 つの値の桁がまったく合わない(片方が実質 0)ピボットを捕まえる。
        // そのまま `basis[r] = q` を確定すると基底行列が特異になり、以後の再計算はすべて
        // 無意味になるので、確定前のここでしか防げない。尺度は `FT_MIN_PIVOT` で下限を
        // 取らない(取ると小さな値同士の桁違いを見逃す)。`1e-300` は 0/0 の回避だけが目的。
        let pivot_grossly_inconsistent = {
            let scale = alpha_q.abs().max(alpha_full[r].abs()).max(GROSS_MISMATCH_SCALE_FLOOR);
            (alpha_q - alpha_full[r]).abs() / scale > D_GROSS_MISMATCH_REL_TOL
        };
        if pivot_grossly_inconsistent || (!update_verify_disabled && lu.update_count() > 0 && !super::pivot_values_agree(alpha_q, alpha_full[r])) {
            if env_str!("ENOMOTO_DEBUG_D_DRIFT_EXT").is_some() {
                eprintln!("DEBUG_D_DRIFT: VERIFY_FAIL at iter={iter_idx} q={q} r={r} alpha_q={alpha_q} alpha_full_r={}", alpha_full[r]);
            }
            if stuck_row == Some(r) {
                stuck_row_streak += 1;
            } else {
                stuck_row = Some(r);
                stuck_row_streak = 1;
            }
            if ban_discarded_candidates {
                if discard_row == Some(r) {
                    if !discard_banned_cols.contains(&q) {
                        discard_banned_cols.push(q);
                    }
                } else {
                    discard_row = Some(r);
                    discard_banned_cols.clear();
                    discard_banned_cols.push(q);
                }
            }
            // ここで捕まえた PRICE/FTRAN の不一致は分解の数値的破綻(`docs/lu_comparison_enomoto_vs_highs.md` §2.4)なので記録する。
            note_numeric_trouble!();
            if profile_phases {
                prof_phases::REFACTOR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if pivot_grossly_inconsistent {
                    prof_phases::REFACTOR_CAUSE_ILLCOND.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                } else {
                    prof_phases::REFACTOR_CAUSE_VERIFY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
            timed!(profile_phases, prof_phases::REFACTOR, {
                lu = refactor_main!();
                let (fresh_base, fresh_slope) = resync_x_b(std, &cache, &nb_status, &lu, phase, &mut lu_scratch, &mut x_b_base, &mut x_b_slope)?;
                rhs_inc_base = fresh_base;
                rhs_inc_slope = fresh_slope;
                rebuild_rows(&mut infeasible_rows, &mut row_dev, m, &row_bounds, &x_b_base, &x_b_slope);
                fresh_d_into(std, &lu, &basis, &basis_pos, &active_cost, &mut fresh_d_cb, &mut lu_scratch, &mut fresh_d_y, &mut d);
                if dse_refresh_on_refactor {
                    dse = super::DseState::from_basis(m, &lu);
                }
            });
            for &j in &touched_cols {
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            if combined_pending {
                // 未反映のフリップ結果: 上の再同期がフリップ後の `nb_status` から `x_B(M)` を
                // 作り直したので、結合右辺のバッファをリセットするだけでよい。
                for &i in &combined_touched {
                    combined_base[i] = 0.0;
                    combined_slope[i] = 0.0;
                    combined_touched_flag[i] = false;
                }
                combined_touched.clear();
            }
            continue;
        }

        // M フラグ列が `M` 側を離れるのは入る列 `q` になるときだけなので、ピボットが実際に
        // 確定したここで一度だけ解消済みにする(破棄されたピボットでは
        // `iters_since_m_progress` をリセットしない)。`delta[q].is_flagged()` なら
        // `q < n_orig` が保証される(スラックは M フラグを持たない)。自由列 (`MSide::Both`) も
        // 入基すれば同様に解消する。
        if profile_phases && q < n_orig && delta[q].is_flagged() {
            let q_was_m = match nb_status[q] {
                Some(NbStatus::Lower) => delta[q].has_lower(),
                Some(NbStatus::Upper) => delta[q].has_upper(),
                Some(NbStatus::Zero) | None => false,
            };
            if q_was_m {
                prof_phases::M_EXIT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        if q < n_orig && delta[q].is_flagged() && !resolved_m[q] {
            resolved_m[q] = true;
            remaining_m_side -= 1;
            iters_since_m_progress = 0;
        }
        // ここに来たのは行 `r` のピボットが確定したとき(破棄分岐は必ず `continue`)なので、
        // この行について追跡していた連続破棄と禁止リストを解除する。
        if stuck_row == Some(r) {
            stuck_row = None;
            stuck_row_streak = 0;
        }
        if discard_row == Some(r) {
            discard_row = None;
            discard_banned_cols.clear();
        }

        // 入る列による `x_B(M)` の増分更新: `theta` は `q` が現在の非基底値から離基行の
        // 目標境界に到達するまでの移動量で、同じ反復のフリップを反映した後の
        // `x_b_base[r]`/`x_b_slope[r]` から計算する。`nb_status[q]` はピボット確定で消える前に
        // ここで読む。行 `r` 自身の新しい値は構成上 `target` に一致するので、下のループではなく
        // `nb_val_q + theta` を直接代入する。
        let old_status_q = nb_status[q].unwrap();
        if old_status_q == NbStatus::Zero {
            n_zero_nonbasic -= 1;
        }
        let nb_val_q = nb_value_affine(&cache, old_status_q, q)?;
        // 離基変数 `basis[r]` の目標境界(`d_dir` が選んだ側。無限側が選ばれることは無い)。
        let target = if d_dir > 0 { cache.lower[basis[r]]? } else { cache.upper[basis[r]]? };
        // 未反映のフリップ結果がある (`combined_pending`) 場合は、それを反映した後の行 `r` の値。
        let x_r = if combined_pending {
            let ca = combined_alpha_base[r];
            let cs = if combined_slope_nonzero { combined_alpha_slope[r] } else { 0.0 };
            if ca != 0.0 || cs != 0.0 {
                let xs = if combined_slope_nonzero { snap_slope(x_b_slope[r] - cs) } else { x_b_slope[r] };
                Affine1::new(x_b_base[r] - ca, xs)
            } else {
                Affine1::new(x_b_base[r], x_b_slope[r])
            }
        } else {
            Affine1::new(x_b_base[r], x_b_slope[r])
        };
        // `theta` の基底・傾き成分(`(x_r - target) / alpha_q`)。
        let theta_base = (x_r.base - target.base) / alpha_q;
        let theta_slope = (x_r.slope - target.slope) / alpha_q;
        // 下のループが触る行の一覧(S6): `alpha`(とフリップ結果)が疎なら、分岐なしの圧縮で
        // 非ゼロ行を昇順に列挙し、それだけを走査する。`0..m` 走査と同じ行を同じ順に訪れるので、
        // `x_B` の値も `InfeasibleRows` の変更順(chuzr のタイブレークに効く)もビット単位で同一。
        // `None` なら全行走査。
        let xb_list_len: Option<usize> = {
            let est = alpha_nnz + if combined_pending { combined_nnz } else { 0 };
            if (est as f64) <= tunable!("ENOMOTO_T_XB_LIST_DENSITY", XB_LIST_DENSITY, f64) * m as f64 {
                // 策13 の一部: フリップ結果と入る列の結果の非ゼロ位置がすべて記録されていれば、その和集合から
                // 非ゼロ行を昇順に集める (`compact_rows` と同じ行の集合・順序)。
                let union_lists = if flip_track && combined_pending {
                    match (ftran_track.alpha.indices(), cab_track.indices(), if combined_slope_nonzero { cas_track.indices() } else { Some(&[][..]) }) {
                        (Some(a), Some(b), Some(c)) => Some((a, b, c)),
                        _ => None,
                    }
                } else {
                    None
                };
                Some(if let Some((la, lb, lc)) = union_lists {
                    xb_union.clear();
                    for &i in la.iter().chain(lb).chain(lc) {
                        let nz = alpha_full[i] != 0.0 || combined_alpha_base[i] != 0.0 || (combined_slope_nonzero && combined_alpha_slope[i] != 0.0);
                        if nz {
                            xb_union.push(i);
                        }
                    }
                    xb_union.sort_unstable();
                    xb_union.dedup();
                    for (k, &i) in xb_union.iter().enumerate() {
                        xb_rows[k] = i as u32;
                    }
                    xb_union.len()
                } else if combined_pending && combined_slope_nonzero {
                    compact_rows(m, &mut xb_rows, |i| (alpha_full[i].to_bits() | combined_alpha_base[i].to_bits() | combined_alpha_slope[i].to_bits()) << 1)
                } else if combined_pending {
                    compact_rows(m, &mut xb_rows, |i| (alpha_full[i].to_bits() | combined_alpha_base[i].to_bits()) << 1)
                } else if let Some(idx) = if BIG { ftran_track.alpha.indices() } else { None } {
                    // 策5: 融合 FTRAN が記録した位置(超疎経路)から非ゼロ行を昇順に集める
                    // (`compact_rows` の `O(m)` 走査の代わり。同じ行の集合・順序)。
                    let mut k = 0usize;
                    for &i in idx {
                        if alpha_full[i] != 0.0 {
                            xb_rows[k] = i as u32;
                            k += 1;
                        }
                    }
                    xb_rows[..k].sort_unstable();
                    k
                } else {
                    compact_rows(m, &mut xb_rows, |i| alpha_full[i].to_bits() << 1)
                })
            } else {
                None
            }
        };
        timed!(profile_phases, prof_phases::XB_UPDATE, {
            if combined_pending {
                // フリップ結果と入る列のステップを 1 パスで: 行ごとに、別パスのフリップ反映と
                // 同じ演算の後に入る列の演算を行い、`refresh_row` を 1 回呼ぶ
                // (両ループに展開するためマクロにしている)。
                macro_rules! flip_step_row {
                    ($i:expr) => {{
                        let i: usize = $i;
                        let ca = combined_alpha_base[i];
                        let cs = if combined_slope_nonzero { combined_alpha_slope[i] } else { 0.0 };
                        let flip = ca != 0.0 || cs != 0.0;
                        let a = alpha_full[i];
                        if flip || a != 0.0 {
                            if flip {
                                x_b_base[i] -= ca;
                                if combined_slope_nonzero {
                                    x_b_slope[i] = snap_slope(x_b_slope[i] - cs);
                                }
                            }
                            if a != 0.0 {
                                x_b_base[i] -= a * theta_base;
                                if theta_slope != 0.0 {
                                    x_b_slope[i] = snap_slope(x_b_slope[i] - a * theta_slope);
                                }
                            }
                            refresh_row(&mut infeasible_rows, &mut row_dev, &row_bounds, &x_b_base, &x_b_slope, i);
                        }
                    }};
                }
                match xb_list_len {
                    Some(k) => {
                        for &i in &xb_rows[..k] {
                            flip_step_row!(i as usize);
                        }
                    }
                    None => {
                        for i in 0..m {
                            flip_step_row!(i);
                        }
                    }
                }
                for &i in &combined_touched {
                    rhs_inc_base[i] -= combined_base[i];
                    // 空の `rhs_inc_slope` は `delta = 0` 表現。非ゼロの傾き寄与が来たときだけ実体化する。
                    if rhs_inc_slope.is_empty() && combined_slope[i] != 0.0 {
                        rhs_inc_slope.resize(m, 0.0);
                    }
                    if !rhs_inc_slope.is_empty() {
                        rhs_inc_slope[i] -= combined_slope[i];
                    }
                    combined_base[i] = 0.0;
                    combined_slope[i] = 0.0;
                    combined_touched_flag[i] = false;
                }
                combined_touched.clear();
            } else if theta_slope == 0.0 {
                // `M` を含まないステップ(delta=0 以降は毎反復、それ以前もほとんど): 傾きチャネルの
                // 更新は正確な no-op(`x_b_slope` は `snap_slope` 済みで `-0.0` も
                // `X_B_SLOPE_NOISE` 未満の値も無い)なので省略する。
                if let Some(k) = xb_list_len {
                    for &i in &xb_rows[..k] {
                        let i = i as usize;
                        x_b_base[i] -= alpha_full[i] * theta_base;
                        refresh_row(&mut infeasible_rows, &mut row_dev, &row_bounds, &x_b_base, &x_b_slope, i);
                    }
                } else {
                    for i in 0..m {
                        let a = alpha_full[i];
                        if a != 0.0 {
                            x_b_base[i] -= a * theta_base;
                            refresh_row(&mut infeasible_rows, &mut row_dev, &row_bounds, &x_b_base, &x_b_slope, i);
                        }
                    }
                }
            } else if let Some(k) = xb_list_len {
                for &i in &xb_rows[..k] {
                    let i = i as usize;
                    let a = alpha_full[i];
                    x_b_base[i] -= a * theta_base;
                    x_b_slope[i] = snap_slope(x_b_slope[i] - a * theta_slope);
                    refresh_row(&mut infeasible_rows, &mut row_dev, &row_bounds, &x_b_base, &x_b_slope, i);
                }
            } else {
                for i in 0..m {
                    let a = alpha_full[i];
                    if a != 0.0 {
                        x_b_base[i] -= a * theta_base;
                        x_b_slope[i] = snap_slope(x_b_slope[i] - a * theta_slope);
                        refresh_row(&mut infeasible_rows, &mut row_dev, &row_bounds, &x_b_base, &x_b_slope, i);
                    }
                }
            }
            x_b_base[r] = nb_val_q.base + theta_base;
            x_b_slope[r] = snap_slope(nb_val_q.slope + theta_slope);
        });

        // このピボットの目的関数への寄与 `theta_q(M) * dj_q`(`dj_q` は `M` に依存しない)。
        // 下の停滞判定で使う。
        let contribution_base = theta_base * dj_q;
        let contribution_slope = theta_slope * dj_q;

        // 部分 `tau` は行一覧での DSE 更新にしか使えない(全行更新になるなら `tau` を全体で求め直す)。
        if BIG && ftran_track.tau_is_partial() && xb_list_len.is_none() {
            tau_ready = false;
        }
        timed!(profile_phases, prof_phases::DSE_UPDATE, {
            if !tau_ready {
                if BIG {
                    ftran_track.tau.set_full();
                }
                timed!(profile_phases, prof_phases::DSE_FTRAN, lu.solve_into(&rho, &mut lu_scratch, &mut tau));
            }
            match xb_list_len {
                // S7: `alpha` の非ゼロ行だけ(`x_B` 更新の一覧。フリップ結果を併合した場合は上位集合)。
                Some(k) => {
                    let wp_old = match rho_list_len {
                        Some(kr) => rho_rows[..kr].iter().map(|&i| rho[i as usize] * rho[i as usize]).sum::<f64>(),
                        None => rho.iter().map(|v| v * v).sum::<f64>(),
                    };
                    dse.update_after_pivot_rows(r, &alpha_full, &tau, wp_old, &xb_rows[..k]);
                }
                None => dse.update_after_pivot(r, &alpha_full, &tau, &rho),
            }
        });
        // 候補短縮リストに、この反復で逸脱または DSE 重みが変わった行(`x_B` 更新の一覧と `r`)を追加する。
        if shortlist_ready && !chuzr_heap_mode {
            if let Some(k) = xb_list_len {
                for &i in xb_rows[..k].iter() {
                    let i = i as usize;
                    if !in_shortlist[i] {
                        in_shortlist[i] = true;
                        shortlist_rows.push(i);
                    }
                }
                if !in_shortlist[r] {
                    in_shortlist[r] = true;
                    shortlist_rows.push(r);
                }
                shortlist_valid = shortlist_rows.len() <= CHUZR_SHORTLIST_MAX_LEN_FACTOR * shortlist_k + CHUZR_SHORTLIST_MAX_LEN_SLACK;
            }
        }
        if profile_work {
            use std::sync::atomic::Ordering::Relaxed;
            prof_phases::STAT_TAU_NNZ.fetch_add(tau.iter().filter(|v| **v != 0.0).count(), Relaxed);
            prof_phases::STAT_ALPHA_NNZ.fetch_add(alpha_full.iter().filter(|v| **v != 0.0).count(), Relaxed);
        }

        // 停滞検出: 目的関数への寄与自体がほぼ 0 のピボットだけを「進展なし」と数える
        // (`(theta_q * dj_q).abs() < STALL_PROGRESS_EPS`)。傾き成分が非ゼロなら確実な進展とみなす。
        // `stall_limit` を超えたら `bland_mode` に入る。
        if contribution_slope == 0.0 && contribution_base.abs() < STALL_PROGRESS_EPS {
            stall_count += 1;
            if stall_count > stall_limit {
                bland_mode = true;
            }
        } else {
            stall_count = 0;
        }

        // 第 2 の巡回防止シグナル: 各ピボットは(わずかでも)進展を報告して `stall_count` を
        // リセットし続けるのに、実行不能行の集合がまったく縮まない長い連続を検出する。
        // 前反復との比較ではなく「これまでの最小の実行不能行数」を更新できたかで判定する
        // (振動するだけの停滞も捕まえるため)。`infeasible_plateau_limit` 反復更新が無ければ
        // `bland_mode` に入る。
        let infeasible_len = infeasible_rows.rows.len();
        let made_infeasible_progress = infeasible_len < best_infeasible_len;
        if made_infeasible_progress {
            best_infeasible_len = infeasible_len;
        }
        // `remaining_m_side` の進展も進展として数える(実行不能行の集合が入れ替わり続けて
        // 最小値を更新できなくても、M 側列の解消が着実に進んでいれば停滞ではない)。
        let made_m_side_progress = remaining_m_side < best_remaining_m_side;
        if made_m_side_progress {
            best_remaining_m_side = remaining_m_side;
        }
        // 策1 の補強 (pds-100 / s250r10 報告の策2、square41 / ex10 報告の策1(b)): 双対目的関数の進展も
        // 進展として数える。最後にリセットしてからの `|contribution_base|` の累積が、求解開始からの累積
        // (目的関数の尺度) の `plateau_obj_rel` 倍を超えたら進展ありとする (傾き成分の寄与は常に進展)。
        // 実行不能行数は最適解の手前まで増減しうるので、目的関数が着実に伸びている健全な求解で
        // Bland 規則に落とさない。寄与が微小なまま行集合も動かない本物の停滞だけを捕まえる。
        plateau_obj_total += contribution_base.abs();
        plateau_obj_acc += contribution_base.abs();
        let made_obj_progress = plateau_obj_rel > 0.0 && (contribution_slope != 0.0 || plateau_obj_acc > plateau_obj_rel * plateau_obj_total.max(1.0));
        if made_infeasible_progress || made_m_side_progress || made_obj_progress {
            plateau_obj_acc = 0.0;
            infeasible_plateau_count = 0;
        } else {
            infeasible_plateau_count += 1;
            if infeasible_plateau_count > infeasible_plateau_limit {
                bland_mode = true;
            }
        }

        // 基底交換: `basis[r]`(`leaving_var`)は `d_dir` が選んだ境界で非基底になり、`q` が入基する。
        let leaving_var = basis[r];
        nb_status[leaving_var] = Some(if d_dir > 0 { NbStatus::Lower } else { NbStatus::Upper });
        basis_pos[leaving_var] = None;
        basis[r] = q;
        basis_pos[q] = Some(r);
        nb_status[q] = None;
        row_bounds.assign(r, q, &cache, &noise_feasible);
        pivot_log.push(PivotRec { r, q, leaving: leaving_var, old_status_q, flips_end: pivot_flips.len() });
        // `rhs_inc_*`(`b - N x_N(M)`)の更新: `q` は `N` を離れ(その項を足し戻す)、
        // 離基変数が新しい境界で `N` に加わる。
        {
            let val_l = nb_value_affine(&cache, nb_status[leaving_var].unwrap(), leaving_var)?;
            for (j, val, sign) in [(q, nb_val_q, 1.0f64), (leaving_var, val_l, -1.0f64)] {
                if val.base != 0.0 {
                    let vb = sign * val.base;
                    for &(i, v) in std.cols.col(j) {
                        rhs_inc_base[i] += v * vb;
                    }
                }
                if val.slope != 0.0 {
                    let vs = sign * val.slope;
                    if rhs_inc_slope.is_empty() {
                        rhs_inc_slope.resize(m, 0.0);
                    }
                    for &(i, v) in std.cols.col(j) {
                        rhs_inc_slope[i] += v * vs;
                    }
                }
            }
        }
        if price_nonbasic_only {
            // 行方向 PRICE 行列の分割(非基底部/基底部)を更新する(HiGHS
            // `HighsSparseMatrix::update` と同じ入れ替え方式): `q` は各行の非基底部から基底部へ、
            // 離基列(固定列でなければ)は逆へ。行内の順序は `a_p` の値に影響しない。
            // 位置は `price_pos_of_col_entry` 索引(S12)から引く。
            /// PRICE 行列の 2 要素 `a`, `b` を入れ替え、位置索引 `price_pos_of_col_entry` も追従させる。
            #[inline(always)]
            fn swap_entries(col: &mut [u32], val: &mut [f64], entry_of_price: &mut [u32], price_pos_of_col_entry: &mut [u32], a: usize, b: usize) {
                col.swap(a, b);
                val.swap(a, b);
                entry_of_price.swap(a, b);
                price_pos_of_col_entry[entry_of_price[a] as usize] = a as u32;
                price_pos_of_col_entry[entry_of_price[b] as usize] = b as u32;
            }
            for (k, &(i, _)) in std.cols.col(q).iter().enumerate() {
                let last = price_nb_end[i] - 1;
                let pos = price_pos_of_col_entry[col_entry_start[q] + k] as usize;
                debug_assert!(pos >= price_start[i] && pos <= last && price_col[pos] as usize == q, "entering column missing from its row's nonbasic PRICE partition");
                swap_entries(&mut price_col, &mut price_val, &mut col_entry_of_price, &mut price_pos_of_col_entry, pos, last);
                price_nb_end[i] = last;
            }
            if std.lb[leaving_var] != std.ub[leaving_var] {
                for (k, &(i, _)) in std.cols.col(leaving_var).iter().enumerate() {
                    let first = price_nb_end[i];
                    let pos = price_pos_of_col_entry[col_entry_start[leaving_var] + k] as usize;
                    debug_assert!(pos >= first && pos < price_start[i + 1] && price_col[pos] as usize == leaving_var, "leaving column missing from its row's basic PRICE partition");
                    swap_entries(&mut price_col, &mut price_val, &mut col_entry_of_price, &mut price_pos_of_col_entry, pos, first);
                    price_nb_end[i] = first + 1;
                }
            }
        }
        if debug_delta0 && delta0_iter.is_none() {
            let all_off_m_side = m_flagged_cols.iter().all(|&j| match nb_status[j] {
                None | Some(NbStatus::Zero) => true,
                Some(NbStatus::Lower) => cache_orig.lower[j].map_or(true, |a| a.slope == 0.0),
                Some(NbStatus::Upper) => cache_orig.upper[j].map_or(true, |a| a.slope == 0.0),
            });
            if all_off_m_side {
                delta0_iter = Some(iter_idx);
                eprintln!(
                    "DEBUG_EXT_DELTA0: at delta=0 (iter={iter_idx}): infeasible_rows={} q_was_m_flagged={} stall_count={stall_count} n_m_flagged={} m_resolved_by_entering={}",
                    infeasible_rows.rows.len(),
                    m_flagged_cols.contains(&q),
                    m_flagged_cols.len(),
                    m_flagged_cols.len() - remaining_m_side
                );
            }
        }
        if debug_delta0 && delta0_iter.is_some() && (iter_idx % 500 == 0) {
            eprintln!(
                "DEBUG_EXT_DELTA0: iter={iter_idx} infeasible_rows={} stall_count={stall_count}",
                infeasible_rows.rows.len()
            );
        }
        // 行 `r` の基底変数が `leaving_var` から `q` に替わったので、新しい変数の境界で
        // 実行可能性を判定し直す(上の `alpha` ループでの判定を上書きする)。
        refresh_row(&mut infeasible_rows, &mut row_dev, &row_bounds, &x_b_base, &x_b_slope, r);
        // 策7: 遅延ヒープに、この反復で逸脱か DSE 重みが変わった行(`x_B` 更新の一覧と `r`)の新しい
        // スコアを積む。一覧が無い(全行走査した)反復は次の反復で作り直す。
        if BIG && chuzr_heap_ready {
            if let Some(k) = xb_list_len {
                timed!(profile_phases, prof_phases::CHUZR, {
                    for i in xb_rows[..k].iter().map(|&i| i as usize).chain(std::iter::once(r)) {
                        chuzr_ver[i] = chuzr_ver[i].wrapping_add(1);
                        if infeasible_rows.contains(i) {
                            chuzr_heap.push(ChuzrEntry { score: Score2::new(row_dev.dev[i], dse.weight(i)), row: i as u32, ver: chuzr_ver[i] });
                        }
                    }
                });
                chuzr_heap_valid = true;
            }
        }
        if chuzr_dense_scan && xb_list_len.is_none() {
            chuzr_scan_next = true;
        }
        if BIG && chuzr_heap_adaptive {
            chuzr_prev_list_len = xb_list_len.unwrap_or(usize::MAX);
        }

        // 双対値の増分更新(Huangfu & Hall §2.2.3): PRICE が触った全列で
        // `d[j] -= theta_d * a_p[j]`。BFRT フリップは `B`/`c_B` を変えないので `d` に影響しない。
        let theta_d = dj_q / alpha_q;
        // 作業 #8 対処 3: 厳密な退化ピボットの連続を数える(シフトは下の `d` 更新の後)。
        // `Zero` (自由列) の入基は比 0 が設計どおり (論文 3.1 節 (iii)) なので退化と数えない。
        let q_was_zero = old_status_q == NbStatus::Zero;
        if dj_q.abs() <= degen_dj_tol && !q_was_zero {
            degen_run += 1;
            diag_degen += 1;
            diag_degen_max_run = diag_degen_max_run.max(degen_run);
        } else {
            degen_run = 0;
        }
        if debug_d_drift_ext && theta_d.abs() > 1e3 {
            eprintln!(
                "DEBUG_D_DRIFT: LARGE theta_d at iter={iter_idx}: q={q} dj_q={dj_q} alpha_q={alpha_q} theta_d={theta_d} touched_cols={} r={r}",
                touched_cols.len()
            );
        }
        timed!(profile_phases, prof_phases::DUAL_UPDATE, {
            // 更新と `a_p` のクリアを 1 パスで行う。
            if BIG {
                // 策3 の密結果モードで打ち消しに置いた `-0.0` は `+ 0.0` で `+0.0` に戻す (従来と同じ `d` の
                // ビット。通常の PRICE では `a_p` は `-0.0` にならないので恒等)。
                if prefetch_on {
                    dual_update_prefetch(&touched_cols, &mut d, &mut a_p, &mut touched, theta_d, prefetch_dist);
                } else {
                    for &j in &touched_cols {
                        d[j] -= theta_d * (a_p[j] + 0.0);
                        a_p[j] = 0.0;
                        touched[j] = false;
                    }
                }
            } else {
                for &j in &touched_cols {
                    d[j] -= theta_d * a_p[j];
                    a_p[j] = 0.0;
                    touched[j] = false;
                }
            }
            touched_cols.clear();
            if price_nonbasic_only {
                // HiGHS `HEkkDual::updateDual` と同じく、入る列と離基列の `d` を直接設定する
                // (`price_nonbasic_only` 参照)。
                d[q] = 0.0;
                d[leaving_var] = -theta_d;
            }
        });
        if degen_shift_run > 0 && degen_run >= degen_shift_run {
            degen_run = 0;
            diag_shift_events += 1;
            diag_shift_cols += shift_degenerate_costs(std, &nb_status, degen_shift_base, &mut active_cost, &mut d);
        }

        // Forrest-Tomlin 増分更新と再分解トリガ: (2) FT 更新がピボットを拒否、
        // (3) eta のフィルが大きすぎる(`XB_CHECK_INTERVAL` ごと)、(4) 更新回数上限、
        // (5) 合成クロック、および `XB_CHECK_INTERVAL` ごとの `x_B(M)` 残差ドリフトと
        // `d` のドリフト。どれかが問題を見つけたときだけ再分解する。
        // `x_B(M)` の残差は両チャネル(基底と傾き)を検査する(比較の多くは傾きで決まるため)。
        // `try_update_precomputed` は同じ反復の BTRAN/FTRAN でキャプチャ済みの
        // `a_tilde_buf`/`e_tilde_buf` を使う。
        since_check += 1;
        let mut need_refactor = timed!(
            profile_phases,
            prof_phases::FT_UPDATE,
            !(if BIG {
                lu.try_update_tracked(r, &a_tilde_buf, &mut ftran_track, &e_tilde_buf, &mut btran_work, tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64))
            } else {
                lu.try_update_precomputed(r, &a_tilde_buf, &e_tilde_buf, tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64))
            })
        );
        if need_refactor {
            // トリガ (2): FT 更新が自らピボットを拒否した(分解が緩すぎる直接の証拠、`docs/lu_comparison_enomoto_vs_highs.md` §2.4)。
            note_numeric_trouble!();
            if profile_phases {
                prof_phases::REFACTOR_CAUSE_TRY_UPDATE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        // トリガ (4)(`ft_max_updates` 参照): 毎反復の無条件チェック。
        if profile_phases {
            prof_phases::MAX_UPDATE_STREAK.fetch_max(lu.update_count(), std::sync::atomic::Ordering::Relaxed);
        }
        if !need_refactor && lu.update_count() > ft_max_updates(m) {
            need_refactor = true;
            if profile_phases {
                prof_phases::REFACTOR_CAUSE_MAX_UPDATES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        // トリガ (5)(`synth_clock_should_refactor` 参照): 毎反復の無条件チェック。
        // 策11 / 報告 P 策5(b): 求解結果の密度で合成クロックの係数を選ぶ ([`SynthDensity`])。入る列の FTRAN 結果の
        // 非ゼロ率の移動平均が `SYNTH_CLOCK_DENSE_FRACTION` 以上なら密、DSE `tau` の非ゼロ率の移動平均が
        // `SYNTH_CLOCK_MID_TAU_FRACTION` 以上なら中程度 (それぞれ `ENOMOTO_T_...`、0 = 無効)。
        let synth_density = {
            let fd = tunable!("ENOMOTO_T_SYNTH_CLOCK_DENSE_FRACTION", SYNTH_CLOCK_DENSE_FRACTION, f64);
            let fm = tunable!("ENOMOTO_T_SYNTH_CLOCK_MID_TAU_FRACTION", SYNTH_CLOCK_MID_TAU_FRACTION, f64);
            if fd > 0.0 && density_col_aq.expected() >= fd {
                SynthDensity::Dense
            } else if fm > 0.0 && density_tau.expected() >= fm {
                SynthDensity::Mid
            } else {
                SynthDensity::Sparse
            }
        };
        if !need_refactor && synth_clock_should_refactor_density(&lu, synth_density) {
            need_refactor = true;
            if profile_phases {
                prof_phases::REFACTOR_CAUSE_CLOCK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        if !need_refactor && since_check >= xb_check_cadence {
            since_check = 0;
            // square41 報告の策2: 分解自体が従来の上限 `FT_BUMP_LIMIT_FACTOR · m` より大きい (基底が密な)
            // 場合は、上限を `FT_BUMP_LU_RATIO · nnz(LU)` にする。そうした問題では FT 更新 1 回の eta が
            // 数千要素になり、`64·m` だと数十反復ごとに再分解していた (square41: 20 反復に 1 回、1 回 179 ms)。
            // Netlib (`nnz(LU)` は最大でも `~35·m`) には掛からない。
            let bump_base_limit = tunable!("ENOMOTO_T_FT_BUMP_LIMIT_FACTOR", FT_BUMP_LIMIT_FACTOR, usize) * m.max(1);
            let bump_lu_ratio = tunable!("ENOMOTO_T_FT_BUMP_LU_RATIO", FT_BUMP_LU_RATIO, f64);
            let bump_limit = if bump_lu_ratio > 0.0 && lu.lu_nnz() >= bump_base_limit {
                bump_base_limit.max((bump_lu_ratio * lu.lu_nnz() as f64) as usize)
            } else {
                bump_base_limit
            };
            let bump_too_big = lu.fill_count() > bump_limit;
            if bump_too_big {
                need_refactor = true;
                if profile_phases {
                    prof_phases::REFACTOR_CAUSE_BUMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            } else {
                // `x_B(M)` の残差ドリフト検査(`XB_CHECK_INTERVAL` ごと。古典法のように
                // `RESIDUAL_CHECK_MULTIPLIER` でさらに間引かない)。`rhs_incremental` なら増分維持した
                // 右辺、そうでなければ新たに計算した右辺と比較する。
                let fresh;
                let (check_rhs_base, check_rhs_slope): (&[f64], &[f64]) = if rhs_incremental {
                    (&rhs_inc_base, &rhs_inc_slope)
                } else {
                    fresh = if phase == Phase::A {
                        (Vec::new(), compute_rhs_slope_only(std, &cache, &nb_status)?)
                    } else {
                        compute_rhs_affine(std, &cache, &nb_status)?
                    };
                    (&fresh.0, &fresh.1)
                };
                // 空の `check_rhs_slope` は `delta = 0` の合図で、そのとき維持中の `x_b_slope` も正確に 0
                // (以後の更新は `theta_slope == 0.0` で省略され、解消ピボットの残りかすは
                // `snap_slope` で消えている)。よって傾きチャネルの残差は正確に 0 で、
                // `residual_norm_affine` はそのチャネルの計算を省いて 0 を返す。
                debug_assert!(
                    !check_rhs_slope.is_empty() || x_b_slope.iter().all(|v| *v == 0.0),
                    "delta = 0 must leave the maintained slope channel at exact zero (iter {iter_idx})"
                );
                // 段階 A では基底チャネルが両辺とも恒等的に 0 なので傾きの残差だけを測る。
                // 1 回の求解内でのエスカレーション(`XB_DRIFT_TOL` 参照): ドリフト起因の再分解が
                // `XB_DRIFT_ESCALATION_STEP` 回起きるごとに許容誤差を `XB_DRIFT_ESCALATION_FACTOR`
                // 倍にする(上限 `XB_DRIFT_TOL_MAX`)。
                let escalation_steps = (drift_trigger_count / tunable!("ENOMOTO_T_XB_DRIFT_ESCALATION_STEP", XB_DRIFT_ESCALATION_STEP, usize)) as i32;
                let mut effective_drift_tol = (xb_drift_tol * XB_DRIFT_ESCALATION_FACTOR.powi(escalation_steps)).min(XB_DRIFT_TOL_MAX);
                let updates = lu.update_count();
                if updates > XB_CHECK_INTERVAL {
                    // 相対下限(`ENOMOTO_XB_DRIFT_REL_K`、既定オフ): 再分解直後の残差 `drift_resid_after_refactor` より
                    // 下には再分解しても下がらない。
                    let rel_k: f64 = tunable!("ENOMOTO_XB_DRIFT_REL_K", XB_DRIFT_REL_K, f64);
                    effective_drift_tol = effective_drift_tol.max((rel_k * drift_resid_after_refactor).min(XB_DRIFT_TOL_MAX));
                }
                // 更新回数の少ない eta ファイルでは、再分解が割に合うまで中程度のドリフトを許す
                // (`ENOMOTO_XB_DRIFT_MIN_UPDATES`、既定オフ)。
                let min_updates: usize = tunable!("ENOMOTO_XB_DRIFT_MIN_UPDATES", XB_DRIFT_MIN_UPDATES, usize);
                if updates < min_updates {
                    let mult: f64 = tunable!("ENOMOTO_XB_DRIFT_MIN_UPDATES_MULT", XB_DRIFT_MIN_UPDATES_MULT, f64);
                    effective_drift_tol = effective_drift_tol.max((mult * effective_drift_tol).min(XB_DRIFT_TOL_MAX));
                }
                // 新規残差の下限(`xb_fresh_floor` の宣言参照)。オフのときは 0 で何もしない。
                if xb_fresh_floor > 0.0 {
                    effective_drift_tol = effective_drift_tol.max(xb_fresh_floor);
                }
                // S2 (`ENOMOTO_XB_DRIFT_SAMPLE = k >= 2`、既定オフ): まず `1/k` の巡回行サンプルで
                // 残差を推定し、推定値が `guard * tol` を超えたときだけ全体の `O(nnz(A_B))` 検査を行う。
                // 再分解後最初の検査は必ず全体で行う(`drift_resid_after_refactor` を記録するため)。
                let force_full = xb_check_full_every > 0 && drift_checks_since_full + 1 >= xb_check_full_every;
                let sample_clear = xb_drift_sample >= 2 && updates > XB_CHECK_INTERVAL && !force_full && {
                    let (sb, ss) = sampled_residual_affine(std, &basis_pos, &x_b_base, &x_b_slope, check_rhs_base, check_rhs_slope, phase == Phase::A, xb_drift_sample, drift_sample_offset);
                    drift_sample_offset = (drift_sample_offset + 1) % xb_drift_sample;
                    if env_str!("ENOMOTO_DEBUG_XB_DRIFT_EXT").is_some() {
                        eprintln!("DEBUG_XB_DRIFT_SAMPLE: iter={iter_idx} est_base={sb:.3e} est_slope={ss:.3e} effective_tol={effective_drift_tol:.3e}");
                    }
                    // 作業 #10 (C): 直近の全体検査で測った丸め誤差の尺度 (`XB_DRIFT_REL_TOL` の相対判定) も許容誤差に含める
                    // (ken-11 は残差が絶対許容誤差を常に超え、相対判定で再分解しない状態が続くので、絶対許容誤差だけの
                    // 比較では標本検査が全体の検査を 1 回も省けなかった)。
                    let (tb, ts) = if xb_drift_rel_tol > 0.0 {
                        (effective_drift_tol.max(xb_drift_rel_tol * drift_last_scale.0), effective_drift_tol.max(xb_drift_rel_tol * drift_last_scale.1))
                    } else {
                        (effective_drift_tol, effective_drift_tol)
                    };
                    sb <= xb_drift_sample_guard * tb && ss <= xb_drift_sample_guard * ts
                };
                if sample_clear {
                    need_refactor = false;
                    drift_checks_since_full += 1;
                } else {
                    drift_checks_since_full = 0;
                    let (resid_base, resid_slope) = if phase == Phase::A {
                        (0.0, residual_norm_slope(std, &basis, &x_b_slope, check_rhs_slope, &mut resid_scratch_slope))
                    } else {
                        residual_norm_affine(std, &basis, &x_b_base, &x_b_slope, check_rhs_base, check_rhs_slope, &mut resid_scratch_base, &mut resid_scratch_slope)
                    };
                    let resid_max = resid_base.max(resid_slope);
                    if updates <= XB_CHECK_INTERVAL {
                        drift_resid_after_refactor = resid_max;
                    }
                    if env_str!("ENOMOTO_DEBUG_XB_DRIFT_EXT").is_some() {
                        eprintln!(
                            "DEBUG_XB_DRIFT: iter={iter_idx} resid_base={resid_base:.3e} resid_slope={resid_slope:.3e} drift_trigger_count={drift_trigger_count} effective_tol={effective_drift_tol:.3e}"
                        );
                    }
                    need_refactor = resid_max > effective_drift_tol;
                    // 丸め誤差の尺度に対する相対判定(`XB_DRIFT_REL_TOL` 参照): 絶対許容誤差を超えても、
                    // 残差が `A_B x_B` の丸めだけで出る大きさ以下なら再分解しても下がらないので再分解しない。
                    // 尺度は絶対判定で発火しそうなときだけ計算する(発火しない限り経路と手間は従来どおり)。
                    if need_refactor && xb_drift_rel_tol > 0.0 {
                        let (scale_base, scale_slope) = if phase == Phase::A {
                            (0.0, residual_scale_affine(std, &basis, &x_b_slope, &[], check_rhs_slope, &[], &mut resid_scratch_slope, &mut resid_scratch_base).0)
                        } else {
                            residual_scale_affine(std, &basis, &x_b_base, &x_b_slope, check_rhs_base, check_rhs_slope, &mut resid_scratch_base, &mut resid_scratch_slope)
                        };
                        need_refactor = resid_base > effective_drift_tol.max(xb_drift_rel_tol * scale_base) || resid_slope > effective_drift_tol.max(xb_drift_rel_tol * scale_slope);
                        drift_last_scale = (scale_base, scale_slope);
                        if env_str!("ENOMOTO_DEBUG_XB_DRIFT_EXT").is_some() {
                            eprintln!("DEBUG_XB_DRIFT_REL: iter={iter_idx} scale_base={scale_base:.3e} scale_slope={scale_slope:.3e} refactor={need_refactor}");
                        }
                    }
                    if need_refactor {
                        drift_trigger_count += 1;
                        // 再分解後最初の検査(FT 更新 `XB_CHECK_INTERVAL` 回以内)で既に超えているなら、
                        // 分解し直しても残差はこの許容誤差まで下がらない(分解自体の誤差が許容誤差と
                        // 同程度)。`XB_DRIFT_ESCALATION_STEP` 回の無駄な再分解を待たず、次の段へ
                        // すぐ緩める(klein3: 再分解 22 → 15 回)。緩め方は既存の段と上限
                        // `XB_DRIFT_TOL_MAX` の範囲に収まる。分解直後の残差を基準にした相対許容誤差
                        // (`ENOMOTO_XB_DRIFT_FRESH_FLOOR`)も試したが、係数 10 で Netlib 93 問 +1.7%、
                        // 100 では pilot87 の目的関数値が狂った。
                        if updates <= XB_CHECK_INTERVAL {
                            let step = tunable!("ENOMOTO_T_XB_DRIFT_ESCALATION_STEP", XB_DRIFT_ESCALATION_STEP, usize);
                            drift_trigger_count = drift_trigger_count.div_ceil(step) * step;
                        }
                        note_numeric_trouble!();
                        if profile_phases {
                            prof_phases::REFACTOR_CAUSE_DRIFT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                }
                // `d` 自身の独立なドリフト検査(`D_DRIFT_TOL` 参照): `x_B(M)` 検査が再分解を
                // 決めていない場合だけ、`since_d_drift_check` の粗い周期で行う。固定列
                // (`lb == ub`、PRICE が `d` を更新しない)は残差から除外する。
                if !need_refactor {
                    since_d_drift_check += 1;
                }
                // `ENOMOTO_D_DRIFT_REFACTOR_ONLY=1`(S18、既定オフ): この周期検査をやめ、`d` の
                // 再同期を再分解時の `fresh_d_into` だけに任せる。
                if !need_refactor && since_d_drift_check >= RESIDUAL_CHECK_MULTIPLIER && tunable!("ENOMOTO_D_DRIFT_REFACTOR_ONLY", 0u8, u8) == 0 {
                    since_d_drift_check = 0;
                    fresh_d_into(std, &lu, &basis, &basis_pos, &active_cost, &mut fresh_d_cb, &mut lu_scratch, &mut fresh_d_y, &mut fresh_d_buf);
                    let mut resid_sq = 0.0f64;
                    let mut scale_sq = 0.0f64;
                    for j in 0..std.n_total {
                        if std.lb[j] == std.ub[j] {
                            continue;
                        }
                        let diff = d[j] - fresh_d_buf[j];
                        resid_sq += diff * diff;
                        scale_sq += fresh_d_buf[j] * fresh_d_buf[j];
                    }
                    let resid_d = resid_sq.sqrt();
                    let scale_d = scale_sq.sqrt().max(1.0);
                    if env_str!("ENOMOTO_DEBUG_D_DRIFT_EXT").is_some() {
                        eprintln!("DEBUG_D_DRIFT: iter={iter_idx} resid_d={resid_d:.3e} scale_d={scale_d:.3e} rel={:.3e}", resid_d / scale_d);
                    }
                    if resid_d > D_DRIFT_TOL * scale_d {
                        need_refactor = true;
                        note_numeric_trouble!();
                        if profile_phases {
                            prof_phases::REFACTOR_CAUSE_D_DRIFT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                }
            }
        }
        if need_refactor {
            shortlist_valid = false;
            chuzr_heap_valid = false;
            if profile_phases {
                prof_phases::REFACTOR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            timed!(profile_phases, prof_phases::REFACTOR, {
                lu = refactor_main!();
                // 完全な再同期: 新しい右辺から `x_B(M)` を解き直し、多数の行の実行可能性が
                // 一度に変わりうるので `InfeasibleRows` も作り直す。
                let (fresh_base, fresh_slope) = resync_x_b(std, &cache, &nb_status, &lu, phase, &mut lu_scratch, &mut x_b_base, &mut x_b_slope)?;
                rhs_inc_base = fresh_base;
                rhs_inc_slope = fresh_slope;
                // 新規残差の下限(`ENOMOTO_XB_DRIFT_FRESH_FLOOR`、既定オフ): 再分解ごとに
                // 新しい分解自体が残す残差を 1 回測る。
                if xb_fresh_floor_factor > 0.0 {
                    let (fb, fs) = if phase == Phase::A {
                        (0.0, residual_norm_slope(std, &basis, &x_b_slope, &rhs_inc_slope, &mut resid_scratch_slope))
                    } else {
                        residual_norm_affine(std, &basis, &x_b_base, &x_b_slope, &rhs_inc_base, &rhs_inc_slope, &mut resid_scratch_base, &mut resid_scratch_slope)
                    };
                    let fresh_resid = fb.max(fs);
                    let steps = (drift_trigger_count / tunable!("ENOMOTO_T_XB_DRIFT_ESCALATION_STEP", XB_DRIFT_ESCALATION_STEP, usize)) as i32;
                    let tol_now = (xb_drift_tol * XB_DRIFT_ESCALATION_FACTOR.powi(steps)).min(XB_DRIFT_TOL_MAX);
                    xb_fresh_floor = if fresh_resid > xb_fresh_floor_frac * tol_now { (xb_fresh_floor_factor * fresh_resid).min(XB_DRIFT_TOL_MAX) } else { 0.0 };
                    if env_str!("ENOMOTO_DEBUG_XB_DRIFT_EXT").is_some() {
                        eprintln!("DEBUG_XB_DRIFT_FRESH: iter={iter_idx} fresh_resid={fresh_resid:.3e} tol={tol_now:.3e} floor={xb_fresh_floor:.3e}");
                    }
                }
                rebuild_rows(&mut infeasible_rows, &mut row_dev, m, &row_bounds, &x_b_base, &x_b_slope);
                fresh_d_into(std, &lu, &basis, &basis_pos, &active_cost, &mut fresh_d_cb, &mut lu_scratch, &mut fresh_d_y, &mut d);
                if dse_refresh_on_refactor {
                    dse = super::DseState::from_basis(m, &lu);
                }
            });
        }

    }

    // `max_iters` を使い切って後処理に達しなかった: 誤った `Infeasible` ではなく
    // `None`(`NotSolved`)を返す。作業 #8 対処 5: 最初の求解なら摂動を掛け直して解き直す(退化の巡回など)。
    if uncertified_restart && !safe_pivot {
        RESTART_BAILOUT.with(|f| f.set(true));
    }
    if profile_phases {
        prof_phases::report(wall_t0.elapsed().as_nanos() as usize);
    }
    if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
        eprintln!(
            "DEBUG_EXT_BAILOUT: max_iters={max_iters} exhausted bland_mode={bland_mode} stall_count={stall_count} remaining_m_side={remaining_m_side} n_m_flagged={}",
            m_flagged_cols.len()
        );
    }
    None
}

/// 後処理(論文 Algorithm 1): 終了判定(命題 6.5 (ii): `z^1 < 0` なら非有界)、cleanup 補題
/// (補題 6.6: 人工的な `M` 側に残る非基底列を、真の境界に対する主比率テストで
/// 高々 `K` 回の操作で取り除く。各操作は目的関数値・双対実行可能性・真の境界での
/// 主実行可能性を保つ)、最後に [`polish_with_true_bounds`] による解の取り出し。
///
/// 引数: `basis`/`basis_pos`/`nb_status` は主ループ終了時の基底状態(更新される)、
/// `delta` は各列の `M` 追跡側、`cache` は元問題の `M`-アフィン境界
/// (`ColCache::build`)、`lu` は現基底の LU 分解(再分解せずに受け取る)。
fn finish(std: &StdForm, basis: &mut [usize], basis_pos: &mut [Option<usize>], nb_status: &mut [Option<NbStatus>], delta: &[MSide], cache: &ColCache, n_orig: usize, lu: sparse_lu::FtLu) -> Option<SimplexResult> {
    // `lu` は主ループ最終反復の分解(現基底に対して正確)をそのまま使う。
    // 現基底での `x_B(M)` と目的関数値 `z(M) = z_B + z_N` を求める。
    let (x_b_base, x_b_slope) = solve_x_b(std, &lu, nb_status, cache)?;
    let c_b: Vec<f64> = basis.iter().map(|&bv| std.c[bv]).collect();
    let z_b = Affine1::new(dot(&c_b, &x_b_base), dot(&c_b, &x_b_slope));
    let mut z_n = Affine1::ZERO;
    for j in 0..std.n_total {
        if let Some(status) = nb_status[j] {
            z_n = z_n.add(nb_value_affine(cache, status, j)?.scale(std.c[j]));
        }
    }
    let z = z_b.add(z_n);
    if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
        eprintln!("DEBUG_EXT: z=({},{}) z0_base={}", z.base, z.slope, z_b.base);
    }
    if z.slope < -Z_SLOPE_TOL {
        return Some(SimplexResult { status: Status::Unbounded, x: None });
    }

    // cleanup 補題(論文の補題 6.6): 自分の人工 `M` 側にある非基底列を、基底変数の真の
    // 有限境界に対する主比率テストで有限側へ動かす。場合 (A): どの行にも塞がれず
    // 有限側に到達(基底は不変でその側に置き直す)、場合 (B): 行 `r` が先に塞ぐ
    // (列が入基し、`basis[r]` が到達した有限境界で離基)。各操作が真の境界での主実行可能性を
    // 保つので、`K` 回で `M` を含まない最適基底解が得られ、polish はほぼ何もしない
    // (数値的安全網と真のコストでの双対チェックとして残す)。
    // `ENOMOTO_LEGACY_CLEANUP=1` で旧来の退化ピボット版 cleanup(A/B 比較用)。
    let mut lu = lu;
    // FT 更新の回数(`FT_CHECK_INTERVAL` 回ごとに再分解)。
    let mut since_check = 0usize;
    let mut lu_scratch = vec![0.0f64; std.n_rows];
    let mut gp_scratch = sparse_lu::GpScratch::new(std.n_rows);
    // cleanup 対象列の `B^-1 A_j`。
    let mut alpha_col = vec![0.0f64; std.n_rows];
    // 入る列の密ベクトル(`try_update` の入力、毎ピボット `col_into_dense` で再充填)。
    let mut dense_j = vec![0.0f64; std.n_rows];
    let debug_ext = env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some();
    // cleanup で実際に行った基底交換の回数。
    let mut cleanup_count = 0usize;
    if env_str!("ENOMOTO_LEGACY_CLEANUP").map_or(true, |v| v == "0") {
        let (mut x_b_base, mut x_b_slope) = (x_b_base, x_b_slope);
        let m = std.n_rows;
        // 列 `j` が人工 `M` 側(`delta` が追跡する側)で非基底か。
        let at_m_side = |j: usize, nb_status: &[Option<NbStatus>]| match nb_status[j] {
            Some(NbStatus::Lower) => delta[j].has_lower(),
            Some(NbStatus::Upper) => delta[j].has_upper(),
            Some(NbStatus::Zero) | None => false,
        };
        // M 側の集合は縮む一方(離基変数は到達した有限境界に置かれる)なので一度だけ集める。
        let pending: Vec<usize> = (0..n_orig).filter(|&j| at_m_side(j, nb_status)).collect();
        // 場合 (A) で置き直した列数(うち `Zero` に置いた自由列の数)。
        let mut parked = 0usize;
        let mut parked_zero = 0usize;
        for &j in &pending {
            let status = nb_status[j]?;
            let v_j = nb_value_affine(cache, status, j)?;
            // `dir`: `x_j` の移動方向(有限側へ。自由列なら 0 へ)、`target`/`target_status`:
            // 移動先の値と非基底状態。移動量を `t` とすると `x_B` は `rate * t`
            // (`rate = -dir * alpha`)だけ変わる。自由列の場合(補題 6.6 (A))は論文の状態 `Z`
            // (値 0、注意 6.7)に置く(`z1 = 0` よりその被約費用は 0 なので双対実行可能)。
            let (dir, target, target_status) = match status {
                NbStatus::Lower => (1.0, if std.ub[j].is_finite() { std.ub[j] } else { 0.0 }, if std.ub[j].is_finite() { NbStatus::Upper } else { NbStatus::Zero }),
                NbStatus::Upper => (-1.0, if std.lb[j].is_finite() { std.lb[j] } else { 0.0 }, if std.lb[j].is_finite() { NbStatus::Lower } else { NbStatus::Zero }),
                NbStatus::Zero => continue,
            };
            // 有限側(または 0)に到達するまでの全移動量(`Affine1`)。
            let full = Affine1::new(target - v_j.base, -v_j.slope).scale(dir);
            lu.solve_sparse_into(std.cols.col(j), &mut lu_scratch, &mut gp_scratch, &mut alpha_col);

            // `Affine1` の移動量に対する Harris 型 2 パス主比率テスト: パス 1 は行ごとの
            // 実行可能性許容誤差だけ緩めた境界で最小移動量を求め、パス 2 はそれ以下で
            // 塞ぐ行のうち |alpha| 最大の行を選ぶ。`blocking(i, relax)` は行 `i` が塞ぐ移動量。
            let blocking = |i: usize, relax: bool| -> Option<Affine1> {
                let a = alpha_col[i];
                if a.abs() <= TOL {
                    return None;
                }
                let rate = -dir * a;
                let bv = basis[i];
                let bound = if rate > 0.0 { std.ub[bv] } else { std.lb[bv] };
                if !bound.is_finite() {
                    return None;
                }
                let tol = if relax { tunable!("ENOMOTO_T_PRIMAL_FEAS_TOL", PRIMAL_FEAS_TOL, f64) * bound.abs().max(1.0) } else { 0.0 };
                let slack = if rate > 0.0 {
                    Affine1::new(bound + tol - x_b_base[i], -x_b_slope[i])
                } else {
                    Affine1::new(x_b_base[i] - (bound - tol), x_b_slope[i])
                };
                Some(slack.scale(1.0 / rate.abs()))
            };
            // パス 1: 緩めた境界での最小の塞ぎ移動量。
            let mut t_max: Option<Affine1> = None;
            for i in 0..m {
                if let Some(t) = blocking(i, true) {
                    if t_max.map_or(true, |b| t.cmp_lex(&b) == std::cmp::Ordering::Less) {
                        t_max = Some(t);
                    }
                }
            }
            // パス 2: 全移動量より先に塞ぐ行があれば、その中で |alpha| 最大の行。
            let row = match t_max {
                Some(tm) if full.cmp_lex(&tm) == std::cmp::Ordering::Greater => {
                    let mut best: Option<(usize, f64)> = None;
                    for i in 0..m {
                        if let Some(t) = blocking(i, false) {
                            if t.cmp_lex(&tm) != std::cmp::Ordering::Greater && best.map_or(true, |(_, ba)| alpha_col[i].abs() > ba) {
                                best = Some((i, alpha_col[i].abs()));
                            }
                        }
                    }
                    best.map(|(i, _)| i)
                }
                _ => None,
            };

            let Some(r) = row else {
                // 場合 (A): 塞がれずに有限側(自由列なら 0)へ到達する。
                for i in 0..m {
                    let a = alpha_col[i];
                    if a != 0.0 {
                        let rate = -dir * a;
                        x_b_base[i] += rate * full.base;
                        x_b_slope[i] = snap_slope(x_b_slope[i] + rate * full.slope);
                    }
                }
                nb_status[j] = Some(target_status);
                parked += 1;
                parked_zero += (target_status == NbStatus::Zero) as usize;
                continue;
            };

            // 場合 (B): 行 `r` が先に塞ぐ — `j` が入基し、`basis[r]` は到達した有限境界で離基する。
            #[cfg(test)]
            CLEANUP_PIVOTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            cleanup_count += 1;
            let t_r = blocking(r, false)?;
            let rate_r = -dir * alpha_col[r];
            for i in 0..m {
                let a = alpha_col[i];
                if a != 0.0 {
                    let rate = -dir * a;
                    x_b_base[i] += rate * t_r.base;
                    x_b_slope[i] = snap_slope(x_b_slope[i] + rate * t_r.slope);
                }
            }
            x_b_base[r] = v_j.base + dir * t_r.base;
            x_b_slope[r] = snap_slope(v_j.slope + dir * t_r.slope);
            let beta_r = basis[r];
            nb_status[beta_r] = Some(if rate_r > 0.0 { NbStatus::Upper } else { NbStatus::Lower });
            basis_pos[beta_r] = None;
            basis[r] = j;
            basis_pos[j] = Some(r);
            nb_status[j] = None;

            std.cols.col_into_dense(j, &mut dense_j);
            since_check += 1;
            let rejected = !lu.try_update(r, &dense_j, tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64));
            if rejected || since_check >= FT_CHECK_INTERVAL {
                since_check = 0;
                lu = refactorize(std, basis_pos, Some(&lu))?;
                (x_b_base, x_b_slope) = solve_x_b(std, &lu, nb_status, cache)?;
                snap_slopes(&mut x_b_slope);
            }
        }
        if debug_ext {
            eprintln!("DEBUG_EXT: cleanup_pivots={cleanup_count} cleanup_parked={parked} cleanup_parked_zero={parked_zero}");
        }
        return polish_with_true_bounds(std, basis, basis_pos, nb_status, lu);
    }
    // 旧来の cleanup(`ENOMOTO_LEGACY_CLEANUP=1`): M 側の非基底列を、`B^-1 A_j` が
    // 非ゼロな最初の行と退化ピボットで入れ替える。
    loop {
        let Some(j) = (0..n_orig).find(|&j| match nb_status[j] {
            Some(NbStatus::Lower) => delta[j].has_lower(),
            Some(NbStatus::Upper) => delta[j].has_upper(),
            Some(NbStatus::Zero) | None => false,
        }) else {
            break;
        };
        // 列 `j` 自身は疎なので超疎 FTRAN(Gilbert-Peierls)で `B^-1 A_j` を求める。
        lu.solve_sparse_into(std.cols.col(j), &mut lu_scratch, &mut gp_scratch, &mut alpha_col);
        let Some(r2) = (0..std.n_rows).find(|&i| alpha_col[i].abs() > TOL) else {
            // `B^{-1}A_j` が恒等的に 0: `j` は入基不要で、値に意味が無いので真の有限側に
            // 直接置く。有限側を持たない自由列がここに来るのは想定外なので `None` で抜ける。
            let Some(side) = (if std.lb[j].is_finite() {
                Some(NbStatus::Lower)
            } else if std.ub[j].is_finite() {
                Some(NbStatus::Upper)
            } else {
                None
            }) else {
                return None;
            };
            nb_status[j] = Some(side);
            continue;
        };
        #[cfg(test)]
        CLEANUP_PIVOTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        cleanup_count += 1;
        let beta_r2 = basis[r2];
        // 追い出す変数は実の(`M` でない)非基底位置を必要とする。自由変数 (`MSide::Both`)
        // を追い出す場合の停止性は示されていないので、`None`(`NotSolved`)で抜ける。
        let Some(true_status) = (if std.lb[beta_r2].is_finite() {
            Some(NbStatus::Lower)
        } else if std.ub[beta_r2].is_finite() {
            Some(NbStatus::Upper)
        } else {
            None
        }) else {
            return None;
        };
        nb_status[beta_r2] = Some(true_status);
        basis_pos[beta_r2] = None;
        basis[r2] = j;
        basis_pos[j] = Some(r2);
        nb_status[j] = None;

        std.cols.col_into_dense(j, &mut dense_j);
        since_check += 1;
        let rejected = !lu.try_update(r2, &dense_j, tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64));
        if rejected || since_check >= FT_CHECK_INTERVAL {
            since_check = 0;
            lu = refactorize(std, basis_pos, Some(&lu))?;
        }
    }

    // 旧来の cleanup は対象外の行の主実行可能性を保たないので、polish の双対再開が
    // 実際に仕事をする。
    if debug_ext {
        eprintln!("DEBUG_EXT: cleanup_pivots={cleanup_count}");
    }
    polish_with_true_bounds(std, basis, basis_pos, nb_status, lu)
}

/// 後処理の最終段: `M`/`Affine1` を使わず `std.lb`/`std.ub` 上で直接動く
/// 通常の有界双対単体法。主フェーズと同じ増分機構(`InfeasibleRows` による
/// 超疎 chuzr、`x_B` の増分維持、Harris 型 BFRT パス 2、確定前の `updateVerify`)を使う。
///
/// DSE/Devex の重み付けは行わない: cleanup が渡す基底ではすでに目的関数値が真の最適値に
/// 一致しており、双対実行可能性を保つ限りこの段のピボットはすべて退化ピボットになるため、
/// 重みは費用を増やすだけで収束を助けない。巡回防止は `perturb_costs` と Bland 規則への
/// 切り替えのみで行う。
///
/// ここでもなお真の片側無限境界を持つ非基底列(例: `<=` 行のスラック)は古典法と同様に
/// 幅が有限でないため、フリップ候補にならず直接のピボット対象にしかならない。
///
/// 引数 `basis`/`basis_pos`/`nb_status` は cleanup 後の基底状態(この関数で更新される)、
/// `lu` はその基底の LU 分解。戻り値 `None` は数値的破綻(`NotSolved` として報告)。
fn polish_with_true_bounds(std: &StdForm, basis: &mut [usize], basis_pos: &mut [Option<usize>], nb_status: &mut [Option<NbStatus>], lu: sparse_lu::FtLu) -> Option<SimplexResult> {
    let n_total = std.n_total;
    let m = std.n_rows;
    // 停滞(目的関数がほぼ進まないピボット)がこの回数を超えたら Bland 規則に切り替える。
    let stall_limit = (STALL_LIMIT_PER_ROW * m).max(STALL_LIMIT_MIN);
    // 連続した停滞ピボットの数。
    let mut stall_count = 0usize;
    // Bland 規則(最小添字優先)による巡回防止モードか。
    let mut bland_mode = false;
    // 摂動済みコスト。この段の双対値 `d` はこれに基づく。主ループと同じ [`dual_active_costs`]
    // (スラック列は真の費用) を使い、主ループが最適と判断した基底がこの段の費用でも双対実行
    // 可能になるようにする (cont1 分析の策5 前半)。`ENOMOTO_POLISH_PERTURB_SLACK=1` で旧動作
    // (`super::perturb_costs` をそのまま使い、スラック列も摂動する。A/B 用)。
    let active_cost = if tunable!("ENOMOTO_POLISH_PERTURB_SLACK", 0u8, u8) != 0 { super::perturb_costs(std) } else { dual_active_costs(std) };

    let mut lu = lu;
    // 前回の FT 更新チェック/再分解からの反復数。
    let mut since_check = 0usize;
    // 前回の残差チェックからの FT チェック回数(`RESIDUAL_CHECK_MULTIPLIER` 回ごとに残差を測る)。
    let mut since_residual_check = 0usize;
    // LU 求解用の作業領域。
    let mut lu_scratch = vec![0.0f64; m];
    // `ENOMOTO_PROF_PHASES_EXT` 診断: この段の再分解回数(うち CLOCK トリガ分)をローカルに数える。
    let profile_phases_polish = env_str!("ENOMOTO_PROF_PHASES_EXT").is_some();
    let mut polish_clock_refactors = 0usize;
    let mut polish_refactors = 0usize;

    // この段は cleanup が残した任意の基底から始まるため、増分更新を始める前に
    // その基底の `c_B` に対する BTRAN を 1 回行って被約費用 `d` を正しく初期化する。
    let mut d = vec![0.0f64; n_total];
    {
        let c_b: Vec<f64> = basis.iter().map(|&bv| active_cost[bv]).collect();
        let mut y = vec![0.0f64; m];
        lu.solve_transpose_into(&c_b, &mut lu_scratch, &mut y);
        for j in 0..n_total {
            let mut dj = active_cost[j];
            for &(i, v) in std.cols.col(j) {
                dj -= v * y[i];
            }
            d[j] = dj;
        }
    }
    if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
        // 開始時の (この段の費用での) 双対実行不能列数。固定列は除く。
        let n_bad = (0..n_total)
            .filter(|&j| {
                std.lb[j] != std.ub[j]
                    && match nb_status[j] {
                        None => false,
                        Some(NbStatus::Lower) => d[j] < -TOL,
                        Some(NbStatus::Upper) => d[j] > TOL,
                        Some(NbStatus::Zero) => d[j].abs() > TOL,
                    }
            })
            .count();
        eprintln!("DEBUG_EXT: polish_start_dual_infeasible_cols={n_bad}");
    }

    // 作業バッファ(ループ前に一度だけ確保し毎反復再利用)、行方向 PRICE、`d` の増分更新は
    // 主フェーズと同じ手法を単一の `f64` チャネルで使う。
    // `a_p`: ピボット行 `rho^T A`。`touched`/`touched_cols`: `a_p` の非ゼロ列の印と一覧。
    // `x_b`: 基底変数の値(基底位置順)。`rho`: `B^-T e_r`。`dense_q`: 入る列の密ベクトル。
    // `alpha_full`: `B^-1 A_q`。`candidates`: 比率テストの候補。
    let mut a_p = vec![0.0f64; n_total];
    let mut touched = vec![false; n_total];
    let mut touched_cols: Vec<usize> = Vec::new();
    let mut x_b = vec![0.0f64; m];
    let mut rho = vec![0.0f64; m];
    let mut dense_q = vec![0.0f64; m];
    let mut alpha_full = vec![0.0f64; m];
    let mut candidates: Vec<Cand> = Vec::new();
    // `try_update_precomputed` 用のキャプチャバッファ(BTRAN/FTRAN の副産物)。
    let mut a_tilde_buf = vec![0.0f64; m];
    let mut e_tilde_buf = vec![0.0f64; m];

    // `solve_sparse_into` 専用の作業領域(`lu_scratch` とは共有しない)。
    let mut sparse_scratch = vec![0.0f64; m];
    let mut gp_scratch = sparse_lu::GpScratch::new(m);

    // FTRAN 結果密度の移動平均(入る列用と BFRT 結合フリップ用で別々)。
    // 入力の非ゼロ数と合わせて密/疎ソルブの切り替えに使う。再分解をまたいで保持する。
    let mut density_col_aq = sparse_lu::FtranDensity::new();
    let mut density_bfrt = sparse_lu::FtranDensity::new();

    // BFRT 結合フリップ: 反転する全列の境界変化量をまとめた右辺 (`combined`)、その非ゼロ行の
    // 印と一覧、FTRAN 結果 (`combined_alpha`)、疎ソルブ入力用バッファ。
    let mut combined = vec![0.0f64; m];
    let mut combined_touched_flag = vec![false; m];
    let mut combined_touched: Vec<usize> = Vec::new();
    let mut combined_alpha = vec![0.0f64; m];
    let mut sparse_buf: Vec<(usize, f64)> = Vec::with_capacity(m);

    // 丸め誤差の範囲内でしか実行不能でないと判定された基底変数の印(変数番号で添字付け)。
    let mut noise_feasible = vec![false; n_total];
    // `ENOMOTO_DISABLE_UPDATE_VERIFY` で updateVerify を無効化するか。
    let update_verify_disabled = env_str!("ENOMOTO_DISABLE_UPDATE_VERIFY").is_some();

    // `x_B` の初期値を一度だけ解き、以後は増分で維持する。
    lu.solve_into(&compute_rhs_plain(std, nb_status), &mut lu_scratch, &mut x_b);
    // 主実行不能な基底行の集合(超疎 chuzr 用)。
    let mut infeasible_rows = InfeasibleRows::new(m);
    infeasible_rows.rebuild(m, |i| row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));

    // S5 (`ENOMOTO_HANDOFF_FLIP=1`、既定オフ): 真のコストで双対実行不能な箱型列を
    // 境界フリップしてからこのループを最大 1 回だけ再開する(再開箇所を参照)。
    // `handoff_flip_mode >= 2` なら反転不能な列があっても反転可能な分だけ反転する。
    let handoff_flip_mode = tunable!("ENOMOTO_HANDOFF_FLIP", 0u8, u8);
    let handoff_flip = handoff_flip_mode != 0;
    // すでに S5 の再開を 1 回行ったか。
    let mut flip_restarted = false;
    // 主単体法への引き継ぎから双対ループへ戻った回数 (cont1 策2)。
    let mut handoff_rounds = 0usize;

    // 作業 #5 M2 (主ループの `infeas_check!` と同じ考え): 実行不能を結論する直前に、(1) 更新済みの分解なら
    // 再分解してやり直し、(2) Farkas の証明([`infeasibility_certified`]、真の境界)が成り立たなければ LU の閾値を
    // 1 段上げて再分解し、上げられなければ誤った `Infeasible` の代わりに `None`(`NotSolved`)を返す。
    // (polish は最終段なので、主ループの M1 のように行を雑音として外すことはしない: 本物の違反を残した解を返しうるため。)
    let polish_noise_c: f64 = tunable!("ENOMOTO_T_NOISE_C", NOISE_C, f64);
    let mut polish_uncertified = 0usize;
    // 作業 #5 M1/M1'(polish 版): `‖rho‖^2` が `NOISE_MIN_SQRT_W^2` を超える行だけ、逸脱と比率テストの候補に
    // 主ループと同じ雑音判定を掛ける(`‖rho‖` は PRICE の走査で求める)。`polish_col_inf_norm` は `‖a_j‖∞`(遅延構築)。
    let polish_noise_min_w: f64 = {
        let s = tunable!("ENOMOTO_T_NOISE_MIN_SQRT_W", NOISE_MIN_SQRT_W, f64);
        s * s
    };
    let mut polish_col_inf_norm: Vec<f64> = Vec::new();
    // 作業 #5 M3(polish 版): 直近の成功した再分解以降のピボットとフリップの記録([`rollback_core`])と、
    // 巻き戻したピボットの `(r, q)` の禁止(比率テストで行 `r` について列 `q` を外す、`usize::MAX` = なし)。
    let pivot_rollback = tunable!("ENOMOTO_T_PIVOT_ROLLBACK", 1u8, u8) != 0;
    let mut pivot_log: Vec<PivotRec> = Vec::new();
    let mut pivot_flips: Vec<usize> = Vec::new();
    let mut polish_bans: Vec<(usize, usize)> = Vec::new();
    let mut polish_rollbacks = 0usize;
    macro_rules! polish_refactor {
        () => {{
            match refactorize(std, basis_pos, Some(&lu)) {
                Some(l) => {
                    pivot_log.clear();
                    pivot_flips.clear();
                    l
                }
                None => {
                    if !pivot_rollback {
                        return None;
                    }
                    let (l, bans) = rollback_core(std, basis, basis_pos, nb_status, &mut pivot_log, &mut pivot_flips, &lu, &mut |_, _| {})?;
                    polish_rollbacks += 1;
                    if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
                        eprintln!("DEBUG_EXT: polish singular refactor -> rolled back pivots (last r={} q={}, banned {}), rollbacks={polish_rollbacks}", bans[0].0, bans[0].1, bans.len());
                    }
                    if polish_rollbacks > ROLLBACK_MAX {
                        return None;
                    }
                    pivot_log.clear();
                    pivot_flips.clear();
                    for b in bans {
                        if !polish_bans.contains(&b) {
                            polish_bans.push(b);
                        }
                    }
                    // 巻き戻したピボットの双対ステップを捨てるため、被約費用を作り直す。
                    polish_fresh_d(std, &l, basis, &active_cost, &mut lu_scratch, &mut d);
                    l
                }
            }
        }};
    }
    macro_rules! polish_infeas_check {
        ($r:expr, $needed:expr) => {{
            let r: usize = $r;
            let needed: f64 = $needed;
            if polish_noise_c > 0.0 || tunable!("ENOMOTO_T_INFEAS_CERTIFY", 1u8, u8) != 0 {
                let debug = env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some();
                if lu.update_count() > 0 {
                    lu = polish_refactor!();
                    lu.solve_into(&compute_rhs_plain(std, nb_status), &mut lu_scratch, &mut x_b);
                    infeasible_rows.rebuild(m, |i| row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));
                    continue;
                }
                let rho_norm = rho.iter().map(|v| v * v).sum::<f64>().sqrt();
                if !infeasibility_certified(std, basis_pos, nb_status, &rho, r, |j| (std.lb[j], std.ub[j])) {
                    polish_uncertified += 1;
                    if debug {
                        eprintln!("DEBUG_EXT: polish infeas_guard: uncertified r={r} needed={needed:.3e} |rho|={rho_norm:.3e} count={polish_uncertified}");
                    }
                    if polish_uncertified > UNCERTIFIED_MAX || !sparse_lu::escalate_pivot_threshold() {
                        RESTART_BAILOUT.with(|f| f.set(true));
                        return None;
                    }
                    lu = polish_refactor!();
                    lu.solve_into(&compute_rhs_plain(std, nb_status), &mut lu_scratch, &mut x_b);
                    infeasible_rows.rebuild(m, |i| row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));
                    continue;
                }
            }
        }};
    }
    let max_iters = super::max_iters_for(m, n_total);
    for iter_idx in 0..max_iters {
        // chuzr: 重みなしの最大逸脱 (Dantzig) 規則。`infeasible_rows.rows` のみを走査する。
        // 同値のときは小さい行番号、`bland_mode` では常に最小行番号を選ぶ。
        let mut best: Option<(usize, i32, f64)> = None;
        for &i in &infeasible_rows.rows {
            let Some((d_dir, mag)) = row_deviation_plain(std, basis, &x_b, &noise_feasible, i) else { continue };
            let better = match best {
                None => true,
                Some((br, _, bmag)) => {
                    if bland_mode {
                        i < br
                    } else if mag > bmag {
                        true
                    } else if mag < bmag {
                        false
                    } else {
                        i < br
                    }
                }
            };
            if better {
                best = Some((i, d_dir, mag));
            }
        }
        #[cfg(debug_assertions)]
        {
            // `InfeasibleRows` が全行の新規走査結果と一致することを確認する。
            let mut fresh: Vec<usize> = (0..m).filter(|&i| row_infeasible_plain(std, basis, &x_b, &noise_feasible, i)).collect();
            let mut maintained: Vec<usize> = infeasible_rows.rows.clone();
            fresh.sort_unstable();
            maintained.sort_unstable();
            debug_assert_eq!(fresh, maintained, "InfeasibleRows drifted from a fresh scan at polish iter {iter_idx}");
        }

        // 主実行不能行が無ければ(摂動コストに対し)最適: 真のコストで双対実行可能性を確かめて返す。
        let Some((r, d_dir, needed)) = best else {
            let mut x = vec![0.0; n_total];
            for j in 0..n_total {
                x[j] = match basis_pos[j] {
                    Some(pos) => x_b[pos],
                    None => match nb_status[j].unwrap() {
                        NbStatus::Lower => std.lb[j],
                        NbStatus::Upper => std.ub[j],
                        NbStatus::Zero => 0.0,
                    },
                };
            }
            if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
                let obj: f64 = (0..n_total).map(|j| std.c[j] * x[j]).sum();
                eprintln!("DEBUG_EXT: polish_iters={iter_idx} bland_mode={bland_mode} obj={obj}");
            }
            // この点は摂動コストに対して最適。コスト摂動が真の双対実行不能性を
            // 隠していないかを、摂動なしのコストで `d` を再計算して確かめる。
            let mut true_d = vec![0.0f64; n_total];
            {
                let c_b: Vec<f64> = basis.iter().map(|&bv| std.c[bv]).collect();
                let mut y = vec![0.0f64; m];
                lu.solve_transpose_into(&c_b, &mut lu_scratch, &mut y);
                for j in 0..n_total {
                    let mut dj = std.c[j];
                    for &(i, v) in std.cols.col(j) {
                        dj -= v * y[i];
                    }
                    true_d[j] = dj;
                }
            }
            // 固定列 (`lb == ub`) はどのピボットでも動かないので、符号の誤った被約費用が
            // あっても実行可能な対処が無く、双対実行不能とは数えない
            // (`ENOMOTO_POLISH_DUAL_KEEP_FIXED` で数える)。`polish_dual_tol` は既定で `TOL`
            // (`ENOMOTO_POLISH_DUAL_TOL` で上書き可)。
            let polish_dual_tol = tunable!("ENOMOTO_POLISH_DUAL_TOL", TOL, f64);
            let exclude_fixed = tunable!("ENOMOTO_POLISH_DUAL_KEEP_FIXED", 0u8, u8) == 0;
            let is_dual_bad = |j: usize| -> bool {
                if exclude_fixed && std.lb[j] == std.ub[j] {
                    return false;
                }
                match nb_status[j] {
                    None => false,
                    Some(NbStatus::Lower) => true_d[j] < -polish_dual_tol,
                    Some(NbStatus::Upper) => true_d[j] > polish_dual_tol,
                    Some(NbStatus::Zero) => true_d[j].abs() > polish_dual_tol,
                }
            };
            let true_dual_feasible = !(0..n_total).any(is_dual_bad);
            let mut t = super::Tableau { std, basis: basis.to_vec(), basis_pos: basis_pos.to_vec(), nb_status: nb_status.to_vec(), x };
            if true_dual_feasible {
                // 残ったのが固定列(または緩めた許容誤差以下)の不整合だけなら、
                // `x_B` を LU から作り直した値をそのまま返す(`run_phase` への引き継ぎ相当)。
                // フィルタなしのチェックでも問題が無ければ `x` をそのまま返す。
                let any_bad_unfiltered = (0..n_total).any(|j| match nb_status[j] {
                    None => false,
                    Some(NbStatus::Lower) => true_d[j] < -TOL,
                    Some(NbStatus::Upper) => true_d[j] > TOL,
                    Some(NbStatus::Zero) => true_d[j].abs() > TOL,
                });
                if any_bad_unfiltered {
                    t.recompute_basics(&lu);
                    if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
                        eprintln!("DEBUG_EXT: polish handoff skipped (only fixed-column/sub-tolerance dual infeasibilities)");
                    }
                    return Some(SimplexResult { status: Status::Optimal, x: Some(t.x[0..t.n_structural()].to_vec()) });
                }
                return Some(SimplexResult { status: Status::Optimal, x: Some(t.x) });
            }
            // S5 (`ENOMOTO_HANDOFF_FLIP=1`、既定オフ): 真のコストでの双対実行不能列が
            // すべて反対側の境界が有限な列なら、それらを反対側へフリップすると基底は真のコストで
            // 双対実行可能になる(代わりに主実行不能になる)。それはこの双対ループが直せるので、
            // 真の被約費用でループを続行する。1 回限りで、2 回目の失敗や反転不能な列
            // (自由列・片側列)がある場合は下の主単体法への引き継ぎへ進む。
            if handoff_flip && !flip_restarted {
                // 反転する列と、反転では直せない列があったか。
                let mut flips: Vec<usize> = Vec::new();
                let mut unflippable = false;
                for j in 0..n_total {
                    if !is_dual_bad(j) {
                        continue;
                    }
                    match nb_status[j] {
                        Some(NbStatus::Lower) if std.ub[j].is_finite() => flips.push(j),
                        Some(NbStatus::Upper) if std.lb[j].is_finite() => flips.push(j),
                        _ => unflippable = true,
                    }
                }
                if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
                    eprintln!("DEBUG_EXT: polish flip candidates={} unflippable={unflippable} at polish_iter={iter_idx}", flips.len());
                }
                if (!unflippable || handoff_flip_mode >= 2) && !flips.is_empty() {
                    for &j in &flips {
                        nb_status[j] = Some(match nb_status[j] {
                            Some(NbStatus::Lower) => NbStatus::Upper,
                            _ => NbStatus::Lower,
                        });
                    }
                    d.copy_from_slice(&true_d);
                    pivot_log.clear();
                    pivot_flips.clear();
                    lu.solve_into(&compute_rhs_plain(std, nb_status), &mut lu_scratch, &mut x_b);
                    noise_feasible.fill(false);
                    infeasible_rows.rebuild(m, |i| row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));
                    stall_count = 0;
                    flip_restarted = true;
                    continue;
                }
            }
            // 摂動が真の双対実行不能性を隠していた: 基底は主実行可能だが真のコストでは
            // 双対実行可能でない。ここからは主単体法の不変条件が必要なので、
            // `super::run_phase` の phase 2 をそのまま再利用する(cleanup 後の非基底列は
            // すべて有限側にあるので特別扱いは不要)。`expand`/`se` は新規状態から始める。
            if handoff_rounds >= HANDOFF_MAX_ROUNDS {
                if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
                    eprintln!("DEBUG_EXT: polish handoff limit ({HANDOFF_MAX_ROUNDS}) reached; still dual infeasible -> NotSolved");
                }
                return None;
            }
            let mut stall = super::PrimalStallState::new();
            // 作業 #5 M3: 主単体法が基底を変えるので巻き戻しの記録を捨てる。
            pivot_log.clear();
            pivot_flips.clear();
            if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
                eprintln!("DEBUG_EXT: polish DUAL->PRIMAL cleanup handoff at polish_iter={iter_idx}");
            }
            // ここでの特異基底は `None`(`NotSolved`)として伝播する。
            let handoff_t0 = std::time::Instant::now();
            let handoff_iters0 = super::prof_phases::RUN_PHASE_ITERS.load(std::sync::atomic::Ordering::Relaxed);
            // `ENOMOTO_HANDOFF_INCREMENTAL` (S4、既定オン、`0` で無効): `run_phase` の代わりに
            // `x_B`/`d` 増分維持版の主単体ループを使う(経路は変わりうる)。
            let status = if tunable!("ENOMOTO_HANDOFF_INCREMENTAL", 1u8, u8) != 0 {
                super::run_phase2_incremental(std, &mut t, &mut lu, &mut stall)
            } else {
                let mut expand = super::ExpandState::new();
                let mut se = super::SteepestEdgeState::new(std);
                super::run_phase(std, &mut t, false, &mut lu, &mut since_check, &mut expand, &mut se, &mut stall)
            };
            if profile_phases_polish {
                eprintln!("PROF_HANDOFF run_phase={:.3}ms ok={}", handoff_t0.elapsed().as_secs_f64() * 1e3, status.is_some());
            }
            let status = status?;
            if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
                let n_bad = (0..n_total).filter(|&j| is_dual_bad(j)).count();
                let handoff_iters = super::prof_phases::RUN_PHASE_ITERS.load(std::sync::atomic::Ordering::Relaxed) - handoff_iters0;
                eprintln!("DEBUG_EXT: primal_handoff_us={} dual_infeasible_cols={n_bad} primal_handoff_iters={handoff_iters}", handoff_t0.elapsed().as_micros());
            }
            // cont1 策1・策2: 主単体法は EXPAND で出る変数を上下限から外れた値のまま非基底にし、
            // 比率テストの行き過ぎが `|alpha|` 倍に増幅されると (cont1 では 6.6e-3) その値のまま
            // "optimal" を返していた。最終基底そのものは正しいので、非基底を `nb_status` の境界値に
            // 戻し (`compute_rhs_plain` は非基底を境界値として読む)、新しい LU で `x_B` を作り直して
            // この双対ループの先頭へ戻る。`x_B` が `PRIMAL_FEAS_TOL` (相対) で実行可能なら先頭の
            // 「実行不能行なし」の分岐が真の費用で双対実行可能性を確かめて返し、実行不能なら
            // 基底は真の費用で双対実行可能なので双対単体法 (真の被約費用) で直す。
            // 引き継ぎは `HANDOFF_MAX_ROUNDS` 回までで、それを超えて再び引き継ぎが要るなら
            // 実行不能・非最適な解を optimal と報告しないよう `NotSolved` (`None`) にする。
            // `ENOMOTO_HANDOFF_RAW_RETURN=1` で旧動作 (主単体法の `x` をそのまま返す、A/B 用)。
            if status == Status::Optimal && tunable!("ENOMOTO_HANDOFF_RAW_RETURN", 0u8, u8) == 0 {
                handoff_rounds += 1;
                basis.copy_from_slice(&t.basis);
                basis_pos.copy_from_slice(&t.basis_pos);
                nb_status.copy_from_slice(&t.nb_status);
                // 主単体法の LU (残差検査つきで保守されている) でまず `x_B` を作り直し、実行可能なら
                // そのまま先頭へ (通常はこちら。Netlib の引き継ぎ 13 問はすべて実行可能)。
                let rhs = compute_rhs_plain(std, nb_status);
                lu.solve_into(&rhs, &mut lu_scratch, &mut x_b);
                noise_feasible.fill(false);
                infeasible_rows.rebuild(m, |i| row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));
                if !infeasible_rows.rows.is_empty() {
                    // 双対ループで直す: 新しい LU で `x_B` と真の費用の被約費用 `d` を作り直す。
                    lu = refactorize(std, basis_pos, Some(&lu))?;
                    lu.solve_into(&rhs, &mut lu_scratch, &mut x_b);
                    infeasible_rows.rebuild(m, |i| row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));
                    let c_b: Vec<f64> = basis.iter().map(|&bv| std.c[bv]).collect();
                    let mut y = vec![0.0f64; m];
                    lu.solve_transpose_into(&c_b, &mut lu_scratch, &mut y);
                    for j in 0..n_total {
                        let mut dj = std.c[j];
                        for &(i, v) in std.cols.col(j) {
                            dj -= v * y[i];
                        }
                        d[j] = dj;
                    }
                    since_check = 0;
                }
                stall_count = 0;
                if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
                    eprintln!("DEBUG_EXT: handoff round {handoff_rounds} snapped; infeasible rows after x_B recompute = {}", infeasible_rows.rows.len());
                }
                continue;
            }
            return Some(SimplexResult {
                status: status.clone(),
                x: if status == Status::Optimal { Some(t.x[0..t.n_structural()].to_vec()) } else { None },
            });
        };

        // BTRAN: `rho = B^-T e_r`。この反復の `try_update_precomputed` 用に
        // `e_tilde_buf` もキャプチャする。
        lu.solve_transpose_unit_capture(r, &mut lu_scratch, &mut rho, &mut e_tilde_buf);

        // 行方向の疎 PRICE: `rho` の非ゼロ行だけを走査して `a_p = rho^T A` を作る
        // (固定列は除外。基底列は除外しない)。`rho_sq` は作業 #5 の雑音判定用の `‖rho‖^2`(`|rho_i| > TOL` の行)。
        let mut rho_sq = 0.0f64;
        for i in 0..m {
            let rv = rho[i];
            if rv.abs() <= TOL {
                continue;
            }
            rho_sq += rv * rv;
            for &(j, v) in std.rows.row(i) {
                if std.lb[j] == std.ub[j] {
                    continue;
                }
                if !touched[j] {
                    touched[j] = true;
                    touched_cols.push(j);
                }
                a_p[j] += rv * v;
            }
        }

        // 作業 #5 M1/M1'(polish 版): 行が数値的に従属に近い(`‖rho‖ > NOISE_MIN_SQRT_W`)ときだけ、逸脱が雑音水準なら
        // 行を雑音として外し、比率テストでは `|alpha_j| <= C eps ‖rho‖ ‖a_j‖∞` の候補を外す(`noise_thr`)。
        let mut noise_thr = if polish_noise_c > 0.0 && rho_sq > polish_noise_min_w { polish_noise_c * f64::EPSILON * rho_sq.sqrt() } else { 0.0 };
        if noise_thr > 0.0 && polish_col_inf_norm.is_empty() {
            polish_col_inf_norm.extend((0..n_total).map(|j| std.cols.col(j).iter().fold(0.0f64, |a, &(_, v)| a.max(v.abs()))));
        }
        // 雑音判定で外した候補があり、残りが空なら判定なしで作り直す(polish は行を外せないので、従来どおり
        // 極小ピボットを使う。特異になれば巻き戻す)。
        loop {
        let mut noise_removed = 0usize;
        // chuzc1: 比率テストの候補(Eligible 集合)を作る。
        candidates.clear();
        // Eligible に含まれる `Zero` 列(最小添字)。
        let mut zero_pick: Option<Cand> = None;
        for &j in &touched_cols {
            let Some(status) = nb_status[j] else { continue };
            let alpha_j = a_p[j];
            if alpha_j.abs() <= TOL {
                continue;
            }
            if noise_thr > 0.0 && alpha_j.abs() <= noise_thr * polish_col_inf_norm[j] {
                noise_removed += 1;
                continue;
            }
            if !polish_bans.is_empty() && polish_bans.contains(&(r, j)) {
                continue;
            }
            let sigma = nb_sigma(status, d_dir as f64, alpha_j);
            let hat_alpha = sigma * alpha_j;
            if (d_dir as f64) * hat_alpha >= 0.0 {
                continue;
            }
            if status == NbStatus::Zero {
                // 主フェーズと同じ規則(論文 3.1 節 (iii)・注意 3.1): Eligible 内の `Zero` 列には
                // 比 0 で直接ピボットし、BFRT は行わない。
                let c = Cand { j, hat_alpha, ratio: 0.0 };
                if zero_pick.map_or(true, |z| j < z.j) {
                    zero_pick = Some(c);
                }
                continue;
            }
            let hat_c = (sigma * d[j]).max(0.0);
            candidates.push(Cand { j, hat_alpha, ratio: hat_c / hat_alpha.abs() });
        }
        if let Some(zc) = zero_pick {
            candidates.clear();
            candidates.push(zc);
        }
        if candidates.is_empty() && noise_removed > 0 {
            noise_thr = 0.0;
            continue;
        }
        break;
        }
        if candidates.is_empty() {
            for &j in &touched_cols {
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            // Eligible が空: 行 `r` の逸脱がその行の丸め誤差程度(RHS の大きさで尺度付け)なら
            // noise_feasible として以後除外し、そうでなければ真に実行不能と結論する。
            if needed <= tunable!("ENOMOTO_T_PRIMAL_FEAS_TOL", PRIMAL_FEAS_TOL, f64) * std.b[r].abs().max(1.0) {
                noise_feasible[basis[r]] = true;
                infeasible_rows.set(r, false);
                continue;
            }
            polish_infeas_check!(r, needed);
            return Some(SimplexResult { status: Status::Infeasible, x: None });
        }
        // `bland_mode` かどうかにかかわらず `(ratio, j)` の昇順に並べる(BFRT の歩進は
        // 比の昇順を前提とする。`j` が同値時の決定的なタイブレークになる)。
        candidates.sort_by(|a, b| a.ratio.total_cmp(&b.ratio).then_with(|| a.j.cmp(&b.j)));

        // BFRT パス 1: 累積フリップ容量 `cum` が必要量 `needed` に達する最初の候補
        // `k_star` を求める。`needed`(LU 経由)と `cum`(容量の和)は別経路の丸めを含むので、
        // 絶対 `TOL` ではなく相対許容誤差 `reach_tol` で比較する。
        let reach_tol = TOL.max(LEX_REL_TOL * needed.abs());
        let mut cum = 0.0f64;
        let mut k_star: Option<usize> = None;
        for (idx, cand) in candidates.iter().enumerate() {
            let width = width_plain(std, cand.j);
            if !width.is_finite() {
                k_star = Some(idx);
                break;
            }
            let new_cum = cum + width * cand.hat_alpha.abs();
            if new_cum >= needed - reach_tol {
                k_star = Some(idx);
                break;
            }
            cum = new_cum;
        }
        // 全候補をフリップしても足りない: 実行不能。
        let Some(k_star) = k_star else {
            for &j in &touched_cols {
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            polish_infeas_check!(r, needed);
            return Some(SimplexResult { status: Status::Infeasible, x: None });
        };

        // Harris 型パス 2: `k_star` から後ろ向きに、比が平坦な窓の中で |ピボット| が
        // 最大の候補を選んでそれでピボットする。
        let mut best_idx = k_star;
        {
            let min_ratio = candidates[k_star].ratio - tunable!("ENOMOTO_T_HARRIS_RATIO_TOL", HARRIS_RATIO_TOL, f64);
            let mut window_start = k_star;
            while window_start > 0 && candidates[window_start - 1].ratio >= min_ratio {
                window_start -= 1;
            }
            let mut best_abs = candidates[k_star].hat_alpha.abs();
            for (idx, cand) in candidates.iter().enumerate().take(k_star + 1).skip(window_start) {
                let abs_a = cand.hat_alpha.abs();
                if abs_a > best_abs {
                    best_abs = abs_a;
                    best_idx = idx;
                }
            }
        }

        // BFRT 結合フリップ: 反転する全候補の境界変化量を 1 本の疎右辺にまとめて
        // 1 回の FTRAN で `x_B` に反映する。反転前の `nb_status` を読むので、
        // 下の反転確定ループより前に実行する。
        for cand in &candidates[..best_idx] {
            let old = nb_status[cand.j].unwrap();
            let width = width_plain(std, cand.j);
            debug_assert_ne!(old, NbStatus::Zero, "a `Zero` column is never flipped");
            let sigma = if old == NbStatus::Lower { 1.0 } else { -1.0 };
            let delta_x = width * sigma;
            for &(i, v) in std.cols.col(cand.j) {
                if !combined_touched_flag[i] {
                    combined_touched_flag[i] = true;
                    combined_touched.push(i);
                }
                combined[i] += v * delta_x;
            }
        }
        if !combined_touched.is_empty() {
            if lu.should_use_dense_solve_tracked(combined_touched.len(), &density_bfrt) {
                let result_nnz = lu.solve_into(&combined, &mut lu_scratch, &mut combined_alpha);
                density_bfrt.record(result_nnz, m);
            } else {
                sparse_buf.clear();
                sparse_buf.extend(combined_touched.iter().map(|&i| (i, combined[i])));
                let result_nnz = lu.solve_sparse_into(&sparse_buf, &mut sparse_scratch, &mut gp_scratch, &mut combined_alpha);
                density_bfrt.record(result_nnz, m);
            }
            // FTRAN のフィルインで入力パターン外にも非ゼロが出るので `0..m` を走査する。
            for i in 0..m {
                if combined_alpha[i] != 0.0 {
                    x_b[i] -= combined_alpha[i];
                    infeasible_rows.set(i, row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));
                }
            }
            for &i in &combined_touched {
                combined[i] = 0.0;
                combined_touched_flag[i] = false;
            }
            combined_touched.clear();
        }
        // 反転を `nb_status` に確定する。
        for cand in &candidates[..best_idx] {
            let old = nb_status[cand.j].unwrap();
            nb_status[cand.j] = Some(match old {
                NbStatus::Lower => NbStatus::Upper,
                NbStatus::Upper => NbStatus::Lower,
                NbStatus::Zero => unreachable!("a `Zero` column is never flipped"),
            });
            pivot_flips.push(cand.j);
        }

        // 入る列 `q`、その被約費用 `dj_q`、PRICE によるピボット要素 `alpha_q`。
        let q = candidates[best_idx].j;
        let dj_q = d[q];
        let alpha_q = a_p[q];

        // FTRAN: `alpha_full = B^{-1}A_q`(密/疎を切り替え)。`try_update_precomputed` 用に
        // `a_tilde_buf` もキャプチャする。
        dense_q.fill(0.0);
        for &(i, v) in std.cols.col(q) {
            dense_q[i] = v;
        }
        if lu.should_use_dense_solve_tracked(std.cols.col(q).len(), &density_col_aq) {
            let result_nnz = lu.solve_into_capture(&dense_q, &mut lu_scratch, &mut alpha_full, &mut a_tilde_buf);
            density_col_aq.record(result_nnz, m);
        } else {
            let result_nnz = lu.solve_sparse_into_capture(std.cols.col(q), &mut sparse_scratch, &mut gp_scratch, &mut alpha_full, &mut a_tilde_buf);
            density_col_aq.record(result_nnz, m);
        }

        // updateVerify: PRICE の値 `alpha_q` と FTRAN の値 `alpha_full[r]` を照合し、
        // 不一致なら再分解・再同期してこの反復をやり直す。
        if !update_verify_disabled && lu.update_count() > 0 && !super::pivot_values_agree(alpha_q, alpha_full[r]) {
            lu = polish_refactor!();
            lu.solve_into(&compute_rhs_plain(std, nb_status), &mut lu_scratch, &mut x_b);
            infeasible_rows.rebuild(m, |i| row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));
            for &j in &touched_cols {
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            continue;
        }

        // 入る列による `x_B` の増分更新(`theta` は `q` の移動量)。
        let old_status_q = nb_status[q].unwrap();
        let nb_val_q = match old_status_q {
            NbStatus::Lower => std.lb[q],
            NbStatus::Upper => std.ub[q],
            NbStatus::Zero => 0.0,
        };
        let target = if d_dir > 0 { std.lb[basis[r]] } else { std.ub[basis[r]] };
        let theta = (x_b[r] - target) / alpha_q;
        for i in 0..m {
            let a = alpha_full[i];
            if a != 0.0 {
                x_b[i] -= a * theta;
                infeasible_rows.set(i, row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));
            }
        }
        x_b[r] = nb_val_q + theta;

        // 停滞検出: 目的関数への寄与 `theta * dj_q` がほぼ 0 のピボットを数える。
        let contribution = theta * dj_q;
        if contribution.abs() < STALL_PROGRESS_EPS {
            stall_count += 1;
            if stall_count > stall_limit {
                bland_mode = true;
            }
        } else {
            stall_count = 0;
        }

        // 基底交換: `basis[r]` が離基して境界に、`q` が入基する。
        let leaving_var = basis[r];
        nb_status[leaving_var] = Some(if d_dir > 0 { NbStatus::Lower } else { NbStatus::Upper });
        basis_pos[leaving_var] = None;
        basis[r] = q;
        basis_pos[q] = Some(r);
        nb_status[q] = None;
        pivot_log.push(PivotRec { r, q, leaving: leaving_var, old_status_q, flips_end: pivot_flips.len() });
        // 行 `r` の基底変数が替わったので、新しい変数で実行可能性を判定し直す。
        infeasible_rows.set(r, row_infeasible_plain(std, basis, &x_b, &noise_feasible, r));

        // 双対値の増分更新: `d[j] -= theta_d * a_p[j]`。
        let theta_d = dj_q / alpha_q;
        for &j in &touched_cols {
            d[j] -= theta_d * a_p[j];
        }
        for &j in &touched_cols {
            a_p[j] = 0.0;
            touched[j] = false;
        }
        touched_cols.clear();

        // Forrest-Tomlin 増分更新と再分解トリガ(主フェーズと同じ方式)。
        since_check += 1;
        let mut need_refactor = !lu.try_update_precomputed(r, &a_tilde_buf, &e_tilde_buf, tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64));
        // トリガ (4): FT 更新回数の上限。
        if !need_refactor && lu.update_count() > ft_max_updates(m) {
            need_refactor = true;
        }
        // トリガ (5): 合成クロック。
        if !need_refactor && synth_clock_should_refactor(&lu) {
            need_refactor = true;
            if profile_phases_polish {
                polish_clock_refactors += 1;
            }
        }
        if !need_refactor && since_check >= FT_CHECK_INTERVAL {
            since_check = 0;
            let bump_too_big = lu.fill_count() > tunable!("ENOMOTO_T_FT_BUMP_LIMIT_FACTOR", FT_BUMP_LIMIT_FACTOR, usize) * m.max(1);
            since_residual_check += 1;
            if bump_too_big {
                need_refactor = true;
            } else if since_residual_check >= RESIDUAL_CHECK_MULTIPLIER {
                since_residual_check = 0;
                let fresh_rhs = compute_rhs_plain(std, nb_status);
                need_refactor = residual_norm(std, basis_pos, &x_b, &fresh_rhs) > FT_RESIDUAL_TOL;
            }
        }
        if need_refactor {
            if profile_phases_polish {
                polish_refactors += 1;
            }
            lu = polish_refactor!();
            lu.solve_into(&compute_rhs_plain(std, nb_status), &mut lu_scratch, &mut x_b);
            infeasible_rows.rebuild(m, |i| row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));
        }
    }
    if profile_phases_polish {
        eprintln!("PROF_PHASES_EXT_POLISH refactor_count={polish_refactors} (clock={polish_clock_refactors})");
    }

    None
}

/// `key(i)` が非ゼロである行 `i < m` を昇順に `out` へ書き出し、その個数を返す。
/// `key(i)` は判定対象の値の `x.to_bits() << 1`(複数ベクトルをまとめるときは OR)で、
/// `±0.0` のときだけゼロになるため、`!= 0.0` 走査が訪れる行と完全に一致する。
/// まず [`COMPACT_ROWS_BLOCK`] 行ずつ分岐なしの OR でまとめて判定し、非ゼロを含む
/// ブロックだけを 1 行ずつ分岐なしで詰める(疎なベクトルで分岐予測が効く)。
#[inline(always)]
fn compact_rows(m: usize, out: &mut [u32], key: impl Fn(usize) -> u64) -> usize {
    let mut k = 0usize;
    let mut i0 = 0usize;
    while i0 + COMPACT_ROWS_BLOCK <= m {
        let mut any = 0u64;
        for i in i0..i0 + COMPACT_ROWS_BLOCK {
            any |= key(i);
        }
        if any != 0 {
            for i in i0..i0 + COMPACT_ROWS_BLOCK {
                out[k] = i as u32;
                k += (key(i) != 0) as usize;
            }
        }
        i0 += COMPACT_ROWS_BLOCK;
    }
    for i in i0..m {
        out[k] = i as u32;
        k += (key(i) != 0) as usize;
    }
    k
}

/// 読み出し用のソフトウェアプリフェッチ (x86_64 のみ。他のアーキテクチャでは何もしない)。
/// 値には影響しない。列添字でランダムに引く大きな配列 (`a_p`・`d` など) の読みを先に出すのに使う。
#[inline(always)]
fn prefetch_read<T>(p: *const T) {
    #[cfg(target_arch = "x86_64")]
    // SAFETY: `_mm_prefetch` はアドレスを読まない (無効なアドレスでも例外にならない) ヒント命令。
    unsafe {
        std::arch::x86_64::_mm_prefetch::<{ std::arch::x86_64::_MM_HINT_T0 }>(p as *const i8)
    }
    #[cfg(not(target_arch = "x86_64"))]
    let _ = p;
}

/// chuzc1 の分岐なし候補フィルタの 1 列分: 候補 `Cand` と、残すか (非基底、`|alpha_j| > TOL`、
/// 比率テストの符号条件) を返す。`d_j` は増分維持している被約費用。
#[inline(always)]
fn chuzc1_filter_one(j: usize, alpha_j: f64, status: Option<NbStatus>, d_j: f64, d_dir_f: f64) -> (Cand, bool) {
    // `c_mask` は `Zero` 列の `hat_c` を 0 にする(不変条件により被約費用が 0 なので比も 0)。
    let (is_nb, sigma, c_mask) = match status {
        Some(NbStatus::Lower) => (true, 1.0, 1.0),
        Some(NbStatus::Upper) => (true, -1.0, 1.0),
        Some(NbStatus::Zero) => (true, nb_sigma(NbStatus::Zero, d_dir_f, alpha_j), 0.0),
        None => (false, 0.0, 0.0),
    };
    let hat_alpha = sigma * alpha_j;
    let hat_c = (sigma * d_j).max(0.0) * c_mask;
    let keep = is_nb & !(alpha_j.abs() <= TOL) & !(d_dir_f * hat_alpha >= 0.0);
    (Cand { j, hat_alpha, ratio: hat_c / hat_alpha.abs() }, keep)
}

/// 列数の多い問題用 (`prefetch_on`): `dist` 列先の `a_p`・`d`・`nb_status` を先読みしながら行う
/// chuzc1 の候補フィルタ (主ループの `chuzc1_filter_col!` と同じ結果)。残した候補数を返す。
/// 小さな問題の主ループのコードを増やさないよう別関数にしている。
#[inline(never)]
fn chuzc1_filter_prefetch(tc: &[usize], a_p: &[f64], nb_status: &[Option<NbStatus>], d: &[f64], cand_scratch: &mut [Cand], d_dir_f: f64, dist: usize) -> usize {
    let mut k = 0usize;
    for idx in 0..tc.len() {
        if let Some(&jn) = tc.get(idx + dist) {
            prefetch_read(a_p.as_ptr().wrapping_add(jn));
            prefetch_read(d.as_ptr().wrapping_add(jn));
            prefetch_read(nb_status.as_ptr().wrapping_add(jn));
        }
        let j = tc[idx];
        let (c, keep) = chuzc1_filter_one(j, a_p[j], nb_status[j], d[j], d_dir_f);
        cand_scratch[k] = c;
        k += keep as usize;
    }
    k
}

/// 列数の多い問題用 (`prefetch_on`): 先読み付きの `d` 更新と `a_p`・`touched` の消去
/// (主ループの `BIG` 経路と同じ演算)。
#[inline(never)]
fn dual_update_prefetch(tc: &[usize], d: &mut [f64], a_p: &mut [f64], touched: &mut [bool], theta_d: f64, dist: usize) {
    for idx in 0..tc.len() {
        if let Some(&jn) = tc.get(idx + dist) {
            prefetch_read(d.as_ptr().wrapping_add(jn));
            prefetch_read(a_p.as_ptr().wrapping_add(jn));
        }
        let j = tc[idx];
        d[j] -= theta_d * (a_p[j] + 0.0);
        a_p[j] = 0.0;
        touched[j] = false;
    }
}

/// 列数の多い問題用 (`prefetch_on`): `dist` 要素先の列の `a_p`・`touched` を先読みしながら行う
/// 行方向 PRICE の 1 行分 (主ループの通常経路と同じ演算・同じ `touched_cols` の順)。
#[inline(never)]
fn price_row_prefetch(a_p: &mut [f64], touched: &mut [bool], touched_cols: &mut Vec<usize>, cols: &[u32], vals: &[f64], rv: f64, dist: usize) {
    let n = cols.len();
    for e in 0..n {
        if e + dist < n {
            let jn = cols[e + dist] as usize;
            prefetch_read(a_p.as_ptr().wrapping_add(jn));
            prefetch_read(touched.as_ptr().wrapping_add(jn));
        }
        let j = cols[e] as usize;
        if !touched[j] {
            touched[j] = true;
            touched_cols.push(j);
        }
        a_p[j] += rv * vals[e];
    }
}

/// square41 報告の策3 (PRICE の密結果モード) の 1 行分: 初到達の判定をせず `a_p` に加算だけ行い、
/// 加算の結果がちょうど 0 になった列には `-0.0` を置いて「触れた」印を残す。
#[inline(never)]
fn price_row_dense(a_p: &mut [f64], cols: &[u32], vals: &[f64], rv: f64) {
    for (&j, &v) in cols.iter().zip(vals) {
        let j = j as usize;
        let new = a_p[j] + rv * v;
        a_p[j] = if new == 0.0 { -0.0 } else { new };
    }
}

/// 2 つの同長ベクトルの内積 `Σ a_i b_i`(先頭から順に加算)。
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum()
}

/// 傾き・切片二段解法の単体テスト(手作りの小さな `StdForm` で各経路を検証する)。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::{CscMat, CsrMat};
    use std::sync::atomic::Ordering::Relaxed;

    /// 各行がスラック項を含んだ疎行リストから `StdForm` を直接組み立てる
    /// (presolve を経由せず、[`solve_slope_intercept_dual`] 単体を検証するため)。
    fn std_form(rows: &[Vec<(usize, f64)>], b: Vec<f64>, c: Vec<f64>, lb: Vec<f64>, ub: Vec<f64>) -> StdForm {
        let n_total = lb.len();
        let n_rows = rows.len();
        assert_eq!(c.len(), n_total);
        assert_eq!(ub.len(), n_total);
        let cols = CscMat::from_rows(rows, n_total);
        StdForm { n_total, n_rows, c, rows: CsrMat::from_rows(rows, n_total), cols, b, lb, ub }
    }

    /// 絶対誤差 `1e-6` 以内で等しいか(テスト用の近似比較)。
    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    /// 自由列 x0 が等式行 1 本だけで有限最適値に達すること(`hat_lower`/`hat_upper` が
    /// 同じ列で両方 M 追跡される場合)を確認する。
    #[test]
    fn genuinely_free_column_reaches_a_finite_optimum_via_one_equality_row() {
        // x0 は自由、x1 ∈ [0,10]、x0 + x1 + s = 5、s ∈ [0,0]。min -x0(= max x0)。
        // x0 = 5 - x1 ∈ [-5, 5] なので最適は x0 = 5, x1 = 0。
        let std = std_form(&[vec![(0, 1.0), (1, 1.0), (2, 1.0)]], vec![5.0], vec![-1.0, 0.0, 0.0], vec![f64::NEG_INFINITY, 0.0, 0.0], vec![f64::INFINITY, 10.0, 0.0]);
        let res = solve_slope_intercept_dual(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 5.0), "x={x:?}");
        assert!(approx(x[1], 0.0), "x={x:?}");
    }

    /// 不等式行にしか現れない 2 本の自由列(presolve::freevar が消せない残余ケース)で、
    /// cleanup の場合 (A) が片方を状態 `Z`(値 0)に置いて最適解を返すことを確認する。
    #[test]
    fn two_free_columns_tied_only_through_opposing_inequality_rows_park_one_at_zero() {
        // x0 - x1 + s0 = 3(s0 >= 0)、-x0 + x1 + s1 = 3(s1 >= 0)、min x0 - x1。
        // 差 x0 - x1 は -3 に固定されるが個々の値は非有界(最適面は直線)なので、
        // どちらかは実の境界を持たないまま非基底に残る。主ループが片方を解決し、
        // 残った方は cleanup の主比率テストで何にも塞がれず、場合 (A) で
        // 状態 `Z`(値 0、論文の注意 6.7)に置かれる。
        let rows = vec![vec![(0, 1.0), (1, -1.0), (2, 1.0)], vec![(0, -1.0), (1, 1.0), (3, 1.0)]];
        let std = std_form(&rows, vec![3.0, 3.0], vec![1.0, -1.0, 0.0, 0.0], vec![f64::NEG_INFINITY, f64::NEG_INFINITY, 0.0, 0.0], vec![f64::INFINITY, f64::INFINITY, f64::INFINITY, f64::INFINITY]);
        let res = solve_slope_intercept_dual(&std, &crate::types::LpOptions::default()).expect("cleanup case (A) parks the free survivor at Zero instead of bailing");
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0] - x[1], -3.0), "x={x:?}");
        assert!(approx(x[0], 0.0) || approx(x[1], 0.0), "one free column is parked at Zero: x={x:?}");
    }

    /// コスト 0 の自由列が crash で `Zero` に置かれ、BFRT なしで最初に入基することを確認する。
    #[test]
    fn zero_cost_free_column_starts_at_zero_and_enters_first() {
        // x0 は自由・コスト 0(論文の式 (4) により crash は -M ではなく `Zero` に置く)、
        // x1 ∈ [0,10] コスト 1、x0 + x1 + s = 5、s ∈ [0,0]。初期の全スラック基底では
        // s = 5 が上限超過。Eligible には x0(Zero、比 0)と x1(比 1)があり、
        // `Zero` 規則(論文 3.1 節 (iii)・注意 3.1)で x0 が BFRT なしに入基して x0 = 5, x1 = 0 で最適。
        // M 側に置かれる列は無いので cleanup は何もしない。
        let std = std_form(&[vec![(0, 1.0), (1, 1.0), (2, 1.0)]], vec![5.0], vec![0.0, 1.0, 0.0], vec![f64::NEG_INFINITY, 0.0, 0.0], vec![f64::INFINITY, 10.0, 0.0]);
        assert_eq!(crash(&std, &super::super::perturb_costs(&std), 2)[0], Some(NbStatus::Zero));
        let before = CLEANUP_PIVOTS.load(Relaxed);
        let res = solve_slope_intercept_dual(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 5.0) && approx(x[1], 0.0), "x={x:?}");
        assert_eq!(CLEANUP_PIVOTS.load(Relaxed), before);
    }

    /// 片側非有界列が最初の反復で直接入基し、cleanup を必要としないことを確認する。
    #[test]
    fn direct_pivot_resolves_unbounded_column_into_the_basis() {
        // x0 ∈ [0, +inf)、x1 ∈ [0,10]、x0 + x1 + s = 5、s ∈ [0,0]。min -x0。
        // 最適は x0=5, x1=0。x0 が最初の反復で直接 `q` に選ばれ、傾きちょうど 0 で
        // 着地するので cleanup は発火しない(`CLEANUP_PIVOTS` 不変)。
        let std = std_form(&[vec![(0, 1.0), (1, 1.0), (2, 1.0)]], vec![5.0], vec![-1.0, 0.0, 0.0], vec![0.0, 0.0, 0.0], vec![f64::INFINITY, 10.0, 0.0]);
        let before = CLEANUP_PIVOTS.load(Relaxed);
        let res = solve_slope_intercept_dual(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 5.0), "x={x:?}");
        assert!(approx(x[1], 0.0), "x={x:?}");
        assert_eq!(CLEANUP_PIVOTS.load(Relaxed), before, "this example shouldn't need any cleanup pivot");
    }

    /// 行が無い (`m == 0`) 問題で、コストが有限側を向く列は有限境界で最適になる。
    #[test]
    fn m_zero_bounded_direction_is_optimal_at_its_finite_bound() {
        // 行なし。x0 ∈ [0, +inf)、コストは有限の下限側を好む(`m == 0` 近道の有界分岐)。
        let std = std_form(&[], vec![], vec![1.0], vec![0.0], vec![f64::INFINITY]);
        let res = solve_slope_intercept_dual(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Optimal);
        assert!(approx(res.x.unwrap()[0], 0.0));
    }

    /// 行が無い問題で、コストが無限側を向く列は非有界と報告される。
    #[test]
    fn m_zero_unbounded_direction_reports_unbounded() {
        // 行なし。x0 ∈ [0, +inf)、コストは無限側を好む(`m == 0` 近道の非有界分岐)。
        let std = std_form(&[], vec![], vec![-1.0], vec![0.0], vec![f64::INFINITY]);
        let res = solve_slope_intercept_dual(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Unbounded);
    }

    /// 非有界列の有無にかかわらず、実行不能な行は `Infeasible` と報告される。
    #[test]
    fn infeasible_row_is_reported_regardless_of_an_unbounded_column() {
        // x0 ∈ [0,+inf)、x1 ∈ [0,5]、x0 + x1 + s = -1、s ∈ [0,0]。
        // x0, x1 >= 0 の和が負になることはないので実行不能。
        let std = std_form(&[vec![(0, 1.0), (1, 1.0), (2, 1.0)]], vec![-1.0], vec![0.0, 0.0, 0.0], vec![0.0, 0.0, 0.0], vec![f64::INFINITY, 5.0, 0.0]);
        let res = solve_slope_intercept_dual(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Infeasible);
    }

    /// どの行にも現れないコスト 0 の片側非有界列が、ピボットなしで有限側に置かれることを確認する。
    #[test]
    fn orphaned_unbounded_column_with_zero_cost_is_cleaned_up_without_a_pivot() {
        // x0 ∈ (-inf, 0](有限側 ub が 0 になるようシフト済み)、どの行にも現れず
        // コスト 0。crash は Lower(人工的な -M 側)に置き、主ループは触れないため、
        // cleanup の「B^{-1}A_j が恒等的に 0」分岐に到達し、実際の基底交換なしに
        // 真の有限側 (ub = 0) に置かれる。x1 ∈ [0,10] の自明な行で m >= 1 を保ち、
        // `m == 0` 近道ではなく主ループ→cleanup の経路を通す。
        let std = std_form(&[vec![(1, 1.0), (2, 1.0)]], vec![3.0], vec![0.0, 0.0, 0.0], vec![f64::NEG_INFINITY, 0.0, 0.0], vec![0.0, 10.0, 0.0]);
        let before = CLEANUP_PIVOTS.load(Relaxed);
        let res = solve_slope_intercept_dual(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 0.0), "x={x:?}");
        assert!(approx(x[1], 3.0), "x={x:?}");
        assert_eq!(CLEANUP_PIVOTS.load(Relaxed), before, "the identically-zero branch never performs an actual pivot swap");
    }

    /// cleanup の主比率テストが塞ぐ行でピボットし(論文の補題 6.6 の場合 (B))、
    /// 主実行可能性を保つことを確認する。
    #[test]
    fn cleanup_ratio_test_pivots_on_the_blocking_row_and_stays_primal_feasible() {
        // 手作りの終了状態(全コスト 0、よって z1 = 0):x0 ∈ [0,+inf) は +M 側で非基底、
        // x1 ∈ [0,+inf)、x2 ∈ [0,10] が基底。
        //   -x0 + x1      + s0 = -3  ->  x1 = M - 3
        //    x0 - x1 + x2 + s1 =  5  ->  x2 = 2
        // x0 を M から 0 へ動かすと x1 が速さ 1 で下がり、M - 3 < M で下限に当たる:
        // 場合 (B) で x0 が行 0 に入基(値 3)、x1 は 0 で離基、x2 は 2 のまま。
        // polish はすでに主実行可能な状態を受け取るはず。
        let rows = vec![vec![(0, -1.0), (1, 1.0), (3, 1.0)], vec![(0, 1.0), (1, -1.0), (2, 1.0), (4, 1.0)]];
        let std = std_form(&rows, vec![-3.0, 5.0], vec![0.0; 5], vec![0.0; 5], vec![f64::INFINITY, f64::INFINITY, 10.0, 0.0, 0.0]);
        let n_orig = 3;
        let mut basis = vec![1usize, 2];
        let mut basis_pos = vec![None, Some(0), Some(1), None, None];
        let mut nb_status = vec![Some(NbStatus::Upper), None, None, Some(NbStatus::Lower), Some(NbStatus::Lower)];
        let delta: Vec<MSide> = (0..5).map(|j| if j < n_orig { delta_of(&std, j) } else { MSide::None }).collect();
        let cache = ColCache::build(&std, n_orig);
        let lu = refactorize(&std, &basis_pos, None).unwrap();
        let before = CLEANUP_PIVOTS.load(Relaxed);
        let res = finish(&std, &mut basis, &mut basis_pos, &mut nb_status, &delta, &cache, n_orig, lu).unwrap();
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 3.0) && approx(x[1], 0.0) && approx(x[2], 2.0), "x={x:?}");
        assert_eq!(CLEANUP_PIVOTS.load(Relaxed), before + 1);
        assert_eq!(basis[0], 0, "x0 must enter at the blocking row");
        assert_eq!(nb_status[1], Some(NbStatus::Lower), "x1 leaves at the bound it reached");
    }

    /// BFRT が有界候補を複数フリップしてから非有界列で実ピボットすること
    /// (結合フリップの増分 `x_B(M)` 更新、`COMBINED_FLIP_COUNT`)を確認する。
    #[test]
    fn bfrt_flips_multiple_bounded_candidates_before_the_real_unbounded_pivot() {
        // x1 ∈ [0,1] コスト 1、x2 ∈ [0,1] コスト 2、x0 ∈ [0,+inf) コスト 100
        // (最後に並ぶよう最も高価にした実際の入基列)。x1 + x2 + x0 + s = 10、s ∈ [0,0]。
        // 全コスト非負なので crash は 3 列とも下限に置く。x1, x2 の幅 (各 1) は行の逸脱 (10)
        // に遠く及ばないので、比の昇順(安い順)に両方が上限へフリップされてから x0 が
        // 実ピボットになる。最適は x1=1, x2=1, x0=8。
        let std = std_form(
            &[vec![(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0)]],
            vec![10.0],
            vec![100.0, 1.0, 2.0, 0.0],
            vec![0.0, 0.0, 0.0, 0.0],
            vec![f64::INFINITY, 1.0, 1.0, 0.0],
        );
        let flips_before = COMBINED_FLIP_COUNT.load(Relaxed);
        let res = solve_slope_intercept_dual(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 8.0), "x={x:?}");
        assert!(approx(x[1], 1.0), "x={x:?}");
        assert!(approx(x[2], 1.0), "x={x:?}");
        assert!(
            COMBINED_FLIP_COUNT.load(Relaxed) - flips_before >= 2,
            "expected the BFRT walk to flip both x1 and x2 before reaching x0"
        );
    }

    /// 作業 #5 M2: 真の Farkas 行(`x0 + s0 = 5`、`x0 ∈ [0, 1]`、`s0 = 0`)は証明済みになり、
    /// 実行可能な右辺(`0.5`)では証明にならないことを確認する。
    #[test]
    fn infeasibility_certificate_accepts_a_farkas_row_and_rejects_a_feasible_one() {
        for (b, expect) in [(5.0, true), (0.5, false), (-3.0, true)] {
            let std = std_form(&[vec![(0, 1.0), (1, 1.0)]], vec![b], vec![1.0, 0.0], vec![0.0, 0.0], vec![1.0, 0.0]);
            // スラック s0 が位置 0 の基底、x0 は非基底(下限)。`rho = B^-T e_0 = [1]`。
            let basis_pos = vec![None, Some(0)];
            let nb_status = vec![Some(NbStatus::Lower), None];
            let got = infeasibility_certified(&std, &basis_pos, &nb_status, &[1.0], 0, |j| (std.lb[j], std.ub[j]));
            assert_eq!(got, expect, "b={b}");
        }
        // 非基底列の境界が無限なら(`x0 >= 0` のみ)`b = 5` は実行可能で、証明にならない。
        let std = std_form(&[vec![(0, 1.0), (1, 1.0)]], vec![5.0], vec![1.0, 0.0], vec![0.0, 0.0], vec![f64::INFINITY, 0.0]);
        assert!(!infeasibility_certified(&std, &[None, Some(0)], &[Some(NbStatus::Lower), None], &[1.0], 0, |j| (std.lb[j], std.ub[j])));
    }

    /// 作業 #5 M1': 雑音水準 `thr ‖a_j‖∞` 以下の候補だけが外れ、順序が保たれることを確認する。
    #[test]
    fn noise_pivot_filter_drops_only_candidates_below_the_noise_level() {
        let rows = vec![vec![(0, 2.0), (1, 1.0), (2, 1.0), (3, 1.0)]];
        let std = std_form(&rows, vec![1.0], vec![0.0; 4], vec![0.0; 4], vec![1.0; 4]);
        let mut norms = Vec::new();
        let mut cands = vec![Cand { j: 0, hat_alpha: 1e-9, ratio: 0.0 }, Cand { j: 1, hat_alpha: -0.5, ratio: 1.0 }, Cand { j: 2, hat_alpha: 1.5e-3, ratio: 2.0 }];
        // 列 0 は ‖a_0‖∞ = 2 なので閾値 2e-3、列 2 は 1e-3。
        assert_eq!(filter_noise_pivots(&std, &mut norms, &mut cands, 1e-3, true), 2, "dry run only counts");
        assert_eq!(cands[0].j, 0, "dry run must not reorder");
        let k = filter_noise_pivots(&std, &mut norms, &mut cands, 1e-3, false);
        assert_eq!(k, 2);
        assert_eq!((cands[0].j, cands[1].j), (1, 2));
    }

    /// 作業 #5 M3: 記録したピボット 2 回とフリップ 1 回を `rollback_core` が戻せること(最後の 1 回を戻して
    /// 分解が得られればそこで止まる)を確認する。
    #[test]
    fn rollback_undoes_the_last_pivot_and_its_trailing_flips() {
        // 2 行: x0 + x1 + s0 = 1、x0 - x1 + s1 = 0(x ∈ [0, 1]、s ∈ [0, inf))。
        let rows = vec![vec![(0, 1.0), (1, 1.0), (2, 1.0)], vec![(0, 1.0), (1, -1.0), (3, 1.0)]];
        let std = std_form(&rows, vec![1.0, 0.0], vec![0.0; 4], vec![0.0; 4], vec![1.0, 1.0, f64::INFINITY, f64::INFINITY]);
        // 全スラック基底から: x0 が位置 0 に入り s0 が下限へ(記録 1)、x1 が位置 1 に入り s1 が下限へ(記録 2)、
        // その後(未確定の反復で)どの列もフリップしていないが、記録 1 の前に x1 を上限へフリップした扱いにする。
        let mut basis = vec![0usize, 1usize];
        let mut basis_pos = vec![Some(0), Some(1), None, None];
        let mut nb_status = vec![None, None, Some(NbStatus::Lower), Some(NbStatus::Lower)];
        let mut log = vec![
            PivotRec { r: 0, q: 0, leaving: 2, old_status_q: NbStatus::Lower, flips_end: 1 },
            PivotRec { r: 1, q: 1, leaving: 3, old_status_q: NbStatus::Upper, flips_end: 1 },
        ];
        let mut flips = vec![1usize];
        let prev = refactorize(&std, &basis_pos, None).expect("basis {x0, x1} is nonsingular");
        let mut undone = Vec::new();
        let (_lu, bans) = rollback_core(&std, &mut basis, &mut basis_pos, &mut nb_status, &mut log, &mut flips, &prev, &mut |rec, _| undone.push(rec.q)).expect("undoing one pivot gives a factorizable basis");
        assert_eq!(bans, vec![(1, 1)]);
        assert_eq!(undone, vec![1]);
        assert_eq!(basis, vec![0, 3]);
        assert_eq!(basis_pos, vec![Some(0), None, None, Some(1)]);
        assert_eq!(nb_status, vec![None, Some(NbStatus::Upper), Some(NbStatus::Lower), None]);
        assert_eq!(log.len(), 1, "only the last pivot is undone");
        assert_eq!(flips, vec![1], "flips before the undone pivot are kept");
    }

    /// 作業 #8 対処 3: 費用シフトは被約費用が双対実行可能側に摂動の大きさ未満しか離れていない非基底列だけを、
    /// 双対実行可能側へずらす(基底列・固定列・`Zero` 列と、すでに十分離れた列は変えない)。
    #[test]
    fn degenerate_cost_shift_moves_only_near_zero_nonbasic_duals_inward() {
        let rows = vec![vec![(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0), (4, 1.0), (5, 1.0)]];
        let inf = f64::INFINITY;
        let std = std_form(&rows, vec![1.0], vec![0.0; 6], vec![0.0, 0.0, 0.0, 2.0, -inf, 0.0], vec![1.0, 1.0, 1.0, 2.0, inf, inf]);
        let nb_status = vec![Some(NbStatus::Lower), Some(NbStatus::Upper), Some(NbStatus::Lower), Some(NbStatus::Lower), Some(NbStatus::Zero), None];
        let base = 1e-6;
        let mut cost = vec![0.0; 6];
        let mut d = vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
        let n = shift_degenerate_costs(&std, &nb_status, base, &mut cost, &mut d);
        assert_eq!(n, 2);
        assert!(d[0] >= base && d[0] < 2.0 * base + 1e-18 && cost[0] == d[0], "Lower column shifted up");
        assert!(d[1] <= -base && cost[1] == d[1], "Upper column shifted down");
        assert_eq!(&d[2..], &[1.0, 0.0, 0.0, 0.0], "far, fixed, Zero and basic columns are untouched");
        assert_eq!(&cost[2..], &[0.0; 4]);
    }

    /// 作業 #8 対処 6: `x_B[r]` と `rho^T rhs` のずれは内積の項の大きさ(1 以上)に対する比で測る。
    #[test]
    fn xb_row_mismatch_is_relative_to_the_dot_product_terms() {
        let rho = [2.0, 0.0, -1.0];
        let rhs = [3.0, 100.0, 1.0];
        assert_eq!(xb_row_mismatch(&rho, None, &rhs, 5.0), 0.0);
        assert!((xb_row_mismatch(&rho, None, &rhs, 5.7) - 0.1).abs() < 1e-12);
        assert!((xb_row_mismatch(&rho, Some(&[0, 2]), &rhs, 5.7) - 0.1).abs() < 1e-12);
        assert!((xb_row_mismatch(&[0.0, 0.0, 0.0], None, &rhs, 0.5) - 0.5).abs() < 1e-12);
    }
}
