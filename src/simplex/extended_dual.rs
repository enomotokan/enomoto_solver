//! BFRT 付き拡張双対単体法。`solve_lp_dual` の唯一の LP エンジンで、presolve 後の `StdForm`
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
use crate::params::extended_dual::{D_DRIFT_TOL, D_GROSS_MISMATCH_REL_TOL, FT_MAX_UPDATES_FACTOR, FT_MAX_UPDATES_FLOOR, PIVOT_ESCALATION_STEP, SLOPE_TOL, SYNTH_CLOCK_FACTOR, SYNTH_CLOCK_MIN_UPDATES, XB_CHECK_INTERVAL, XB_DRIFT_ESCALATION_FACTOR, XB_DRIFT_ESCALATION_STEP, XB_DRIFT_TOL, XB_DRIFT_TOL_MAX, X_B_SLOPE_NOISE, Z_SLOPE_TOL};
use crate::params::extended_dual::{
    CHUZR_SHORTLIST_K, CHUZR_SHORTLIST_MAX_LEN_FACTOR, CHUZR_SHORTLIST_MAX_LEN_SLACK, CHUZR_SHORTLIST_MIN_POOL_FACTOR, COMPACT_ROWS_BLOCK, FTRAN_U_HYPER_DENSITY, FTRAN_U_HYPER_TAU_DENSITY,
    GREATEST_IMPROVEMENT_STALL_DIVISOR, GREATEST_IMPROVEMENT_STALL_MIN, GREATEST_IMPROVEMENT_TOP_K, GROSS_MISMATCH_SCALE_FLOOR, INFEASIBLE_PLATEAU_BUDGET_DIVISOR, INFEASIBLE_PLATEAU_STALL_MULT,
    LEX_REL_TOL, PRICE_COLUMN_DENSITY, PRICE_LIST_DENSITY, SCORE2_STALL_HALFLIFE, STALL_LIMIT_MIN, STALL_LIMIT_PER_ROW, STUCK_ROW_BOOST_FACTOR, STUCK_ROW_BOOST_THRESHOLD, XB_DRIFT_FRESH_FLOOR_FACTOR,
    XB_DRIFT_FRESH_FLOOR_FRAC, XB_DRIFT_MIN_UPDATES, XB_DRIFT_MIN_UPDATES_MULT, XB_DRIFT_REL_K, XB_DRIFT_SAMPLE_GUARD, XB_DRIFT_SAMPLE_K, XB_LIST_DENSITY,
};

/// `ENOMOTO_PROF_PHASES_EXT` 診断用のフェーズ別計時カウンタ(`simplex::prof_phases` の拡張版)。
/// [`solve_lp_dual_extended`] の主ループの時間がどこで使われるかを測る。値はすべてナノ秒または
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
    lu.update_count() >= tunable!("ENOMOTO_T_SYNTH_CLOCK_MIN_UPDATES", SYNTH_CLOCK_MIN_UPDATES, usize) && (lu.synth_tick() as f64) >= synth_clock_factor() * (lu.build_tick().max(1) as f64)
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
/// パニックではなく `?` で [`solve_lp_dual_extended`] の `None`(`NotSolved`)まで伝播させる
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
    }
    let mut rows = ROWS.with(|r| std::mem::take(&mut *r.borrow_mut()));
    rows.truncate(m);
    for row in rows.iter_mut() {
        row.clear();
    }
    rows.resize_with(m, Vec::new);
    // `std.cols` による列駆動の構築(`nnz(A_B)` の手間)。`j` を昇順に訪れるので各行の要素順は
    // 行駆動と同一で、Markowitz のタイブレークを含め分解結果は変わらない。
    for j in 0..std.n_total {
        if let Some(col) = basis_pos[j] {
            for &(i, v) in std.cols.col(j) {
                rows[i].push((col, v));
            }
        }
    }
    let r = sparse_lu::factorize_diagonal(m, &rows)
        .map(sparse_lu::FtLu::new)
        .or_else(|| sparse_lu::factorize_reusing(m, &rows, prev));
    ROWS.with(|r| *r.borrow_mut() = rows);
    if r.is_none() && env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
        eprintln!("DEBUG_EXT_BAILOUT: refactorize returned None (singular basis)");
    }
    r
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

/// [`residual_norm`] の 2 チャネル版: `x_B(M)` のドリフト検査用に、
/// `(‖A_B x_b_base - rhs_base‖, ‖A_B x_b_slope - rhs_slope‖)` を返す。基底列だけを
/// 1 回走査する(`nnz(A_B)` の手間)。`basis` は基底位置 → 変数番号。
/// `scratch_base`/`scratch_slope` は呼び出し側の長さ `m` の作業領域で、使用前に上書きする。
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
    scratch_base.iter_mut().for_each(|v| *v = 0.0);
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
            resid_base_sq += rb * rb;
        }
        return (resid_base_sq.sqrt(), 0.0);
    }
    scratch_slope.iter_mut().for_each(|v| *v = 0.0);
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
        resid_base_sq += rb * rb;
        resid_slope_sq += rs * rs;
    }
    (resid_base_sq.sqrt(), resid_slope_sq.sqrt())
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
    scratch.iter_mut().for_each(|v| *v = 0.0);
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

/// 拡張双対単体法の本体(論文 6 節・7 節、Algorithm 1)。数値の `M` を固定せずに、`M` 切り詰め問題で
/// 主実行可能な状態(段階 A: 傾き問題 → 段階 B: 切片問題)に到達し、その後
/// [`finish`] で終了判定・cleanup・真の境界での仕上げを行う。
///
/// 引数: `std` は presolve 後の標準形 LP(連結成分 1 つ分)、`opts` は LP オプション
/// (`distinguish_infeasible_unbounded` が偽なら `z^1 < 0` の時点で
/// `InfeasibleOrUnbounded` を返す)。戻り値 `None` は「到達しないはず」の数値的破綻や
/// 反復上限到達で、呼び出し側が `Status::NotSolved` として報告する。
pub fn solve_lp_dual_extended(std: &StdForm, opts: &crate::types::LpOptions) -> Option<SimplexResult> {
    // 全列数(構造列+スラック列)、行数、構造列数。スラック列は `n_orig..n_total`。
    let n_total = std.n_total;
    let m = std.n_rows;
    let n_orig = n_total - m;

    // コスト摂動(`super::perturb_costs`)を再利用する。`d` の増分維持と `crash` はこの
    // 摂動済みコストに基づく。
    let mut active_cost = super::perturb_costs(std);
    // スラック列は正確な(0 の)コストのまま: スラックを摂動すると全スラック基底でも
    // `y = B^-T c_B` が非ゼロになり、`active_cost` の符号で置く `crash` が双対実行不能から
    // 始まってしまう。摂動しなければ開始時 `y = 0` で crash は構成上双対実行可能。
    for j in n_orig..n_total {
        active_cost[j] = std.c[j];
    }

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
    let mut greatest_improvement_cands: Vec<(usize, i32, Affine1, Score2)> = Vec::with_capacity(GREATEST_IMPROVEMENT_TOP_K * 4);
    // 実行不能行数プラトー検出(ループ本体の該当箇所参照)の上限反復数。健全だが遅い求解の
    // 通常の揺らぎを避けるため `stall_limit` より大きくするが、反復上限の予算内で発火できる
    // よう `MAX_ITERS_FLOOR / 4` で頭打ちにする。
    let infeasible_plateau_limit = (INFEASIBLE_PLATEAU_STALL_MULT * stall_limit).min(MAX_ITERS_FLOOR / INFEASIBLE_PLATEAU_BUDGET_DIVISOR);
    // 最小の実行不能行数を更新できていない連続反復数。
    let mut infeasible_plateau_count = 0usize;
    // これまでに見た最小の実行不能行数(前反復の値ではない)。
    let mut best_infeasible_len = infeasible_rows.rows.len();
    // [`XB_DRIFT_TOL`](エスカレーションの出発点)の上書き(`ENOMOTO_XB_DRIFT_TOL`、A/B 用)。
    let xb_drift_tol: f64 = env_str!("ENOMOTO_XB_DRIFT_TOL").and_then(|s| s.parse::<f64>().ok()).unwrap_or(XB_DRIFT_TOL);
    // この求解内でのドリフト起因の再分解回数([`XB_DRIFT_TOL`] の段階的緩和に使う)。
    let mut drift_trigger_count: usize = 0;
    // 再分解後最初のドリフト検査で測った残差(その分解自体の雑音水準)。
    let mut drift_resid_after_refactor: f64 = 0.0;
    // S2(`ENOMOTO_XB_DRIFT_SAMPLE`、既定オフ): ドリフト残差の巡回行サンプリングによる事前検査
    // ([`sampled_residual_affine`])。`k`(0 でオフ)、ガード係数、次に使う行オフセット。
    let xb_drift_sample: usize = tunable!("ENOMOTO_XB_DRIFT_SAMPLE", XB_DRIFT_SAMPLE_K, usize);
    let xb_drift_sample_guard: f64 = tunable!("ENOMOTO_XB_DRIFT_SAMPLE_GUARD", XB_DRIFT_SAMPLE_GUARD, f64);
    let mut drift_sample_offset: usize = 0;
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
    /// 数値的原因による再分解を 1 回記録し、[`PIVOT_ESCALATION_STEP`] 回ごとに LU の
    /// ピボット閾値を引き上げる。(ループ本体が多くの局所変数を可変借用しているため、
    /// クロージャではなくマクロにしている。)
    macro_rules! note_numeric_trouble {
        () => {{
            numeric_trouble_count += 1;
            if pivot_escalation_step != 0 && numeric_trouble_count % pivot_escalation_step == 0 {
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
    let shortlist_k = tunable!("ENOMOTO_T_CHUZR_SHORTLIST", CHUZR_SHORTLIST_K, usize);
    let shortlist_enabled = shortlist_k > 0 && merge_flip_xb && score2_max_tol == LEX_REL_TOL && stuck_row_boost_factor == 1.0;
    // 短縮リストの行一覧、リスト所属の印、全走査時の上位 `K+1` 行、カットのスコア、有効か。
    let mut shortlist_rows: Vec<usize> = Vec::new();
    let mut in_shortlist = vec![false; if shortlist_enabled { m } else { 0 }];
    let mut shortlist_top: Vec<(Score2, usize)> = Vec::with_capacity(shortlist_k + 2);
    let mut shortlist_cut: Option<Score2> = None;
    let mut shortlist_valid = false;
    // 反復上限(`super::max_iters_for`)。
    let max_iters = super::max_iters_for(m, n_total);
    // ===== 主ループ(1 反復 = chuzr → BTRAN → PRICE → chuzc1/BFRT → FTRAN → 更新) =====
    for iter_idx in 0..max_iters {
        // 前反復終了時点で候補短縮リストが有効だったか(この反復では一旦無効にする)。
        let shortlist_was_valid = shortlist_valid;
        shortlist_valid = false;
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
        // この反復で候補短縮リスト(S11)を使うか。
        let shortlist_active = shortlist_enabled && !bland_mode;
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
            }
            if shortlist_active {
                // 短縮リストを作り直す: プール中の上位 `K + 1` 行。
                shortlist_top.clear();
                let before = |score: &Score2, i: usize, t: &(Score2, usize)| match score.cmp_lex(&t.0, score2_c2_tol) {
                    std::cmp::Ordering::Greater => true,
                    std::cmp::Ordering::Less => false,
                    std::cmp::Ordering::Equal => i < t.1,
                };
                for &i in &infeasible_rows.rows {
                    let score = Score2::new(row_dev.dev[i], dse.weight(i));
                    // 早期棄却: 現在の (K+1) 番目より良くない。
                    if shortlist_top.len() > shortlist_k && !before(&score, i, &shortlist_top[shortlist_k]) {
                        continue;
                    }
                    let pos = shortlist_top.iter().position(|(ts, ti)| match score.cmp_lex(ts, score2_c2_tol) {
                        std::cmp::Ordering::Greater => true,
                        std::cmp::Ordering::Less => false,
                        std::cmp::Ordering::Equal => i < *ti,
                    });
                    match pos {
                        Some(p) => {
                            shortlist_top.insert(p, (score, i));
                            shortlist_top.truncate(shortlist_k + 1);
                        }
                        None if shortlist_top.len() <= shortlist_k => shortlist_top.push((score, i)),
                        None => {}
                    }
                }
                for &i in &shortlist_rows {
                    in_shortlist[i] = false;
                }
                shortlist_rows.clear();
                shortlist_cut = if shortlist_top.len() > shortlist_k { Some(shortlist_top[shortlist_k].0) } else { None };
                for &(_, i) in shortlist_top.iter().take(shortlist_k) {
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
            let mut fresh: Vec<usize> = (0..m).filter(|&i| row_infeasible_affine(&cache, &basis, &x_b_base, &x_b_slope, &noise_feasible, i)).collect();
            let mut maintained: Vec<usize> = infeasible_rows.rows.clone();
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
                greatest_improvement_cands.clear();
                for &i in &infeasible_rows.rows {
                    let (d_dir_i, dev_i) = (row_dev.dir[i], row_dev.dev[i]);
                    let score = Score2::new(dev_i, dse.weight(i));
                    greatest_improvement_cands.push((i, d_dir_i, dev_i, score));
                }
                greatest_improvement_cands.sort_by(|a, b| b.3.cmp_lex(&a.3, score2_c2_tol));
                greatest_improvement_cands.truncate(GREATEST_IMPROVEMENT_TOP_K);

                let mut best_gain: Option<Affine1> = None;
                let mut best_gi: Option<(usize, i32, Affine1)> = None;
                for &(i, d_dir_i, dev_i, _) in &greatest_improvement_cands {
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
            return finish(std, &mut basis, &mut basis_pos, &mut nb_status, &delta, &cache_orig, n_orig, lu);
        };

        // (c): ピボット行の BTRAN `rho = B^-T e_r`(`M` に依存しないので `rho`/`a_p`/`d` は
        // 通常の `f64`)。この反復の `try_update_precomputed` 用に `e_tilde_buf`
        // (U^-T 後・R 逆適用前の中間値)を副産物としてキャプチャする。
        timed!(profile_phases, prof_phases::BTRAN, lu.solve_transpose_unit_work(r, &mut rho, &mut e_tilde_buf, &mut btran_work, Some(&mut rho_steps)));
        // 診断: 維持している DSE 重みと `‖rho‖^2`(真の値)の相対誤差を区間別に数える。
        if profile_phases {
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
                        for (&j, &v) in price_col[lo..hi].iter().zip(&price_val[lo..hi]) {
                            let j = j as usize;
                            if !touched[j] {
                                touched[j] = true;
                                touched_cols.push(j);
                            }
                            a_p[j] += rv * v;
                        }
                    }
                }};
            }
            if price_by_col_now {
                n_priced = rho_count_for_col;
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
        });
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

        // Eligible 内の `Zero` 列(あれば BFRT を行わずこれにピボットする)。
        let mut zero_pick: Option<Cand>;
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
            for &j in &touched_cols {
                let alpha_j = a_p[j];
                // `c_mask` は `Zero` 列の `hat_c` を 0 にする(不変条件により被約費用が 0 なので比も 0)。
                let (is_nb, sigma, c_mask) = match nb_status[j] {
                    Some(NbStatus::Lower) => (true, 1.0, 1.0),
                    Some(NbStatus::Upper) => (true, -1.0, 1.0),
                    Some(NbStatus::Zero) => (true, nb_sigma(NbStatus::Zero, d_dir_f, alpha_j), 0.0),
                    None => (false, 0.0, 0.0),
                };
                let hat_alpha = sigma * alpha_j;
                // `d[j]` は増分維持している被約費用。
                let hat_c = (sigma * d[j]).max(0.0) * c_mask;
                cand_scratch[k] = Cand { j, hat_alpha, ratio: hat_c / hat_alpha.abs() };
                let keep = is_nb & !(alpha_j.abs() <= TOL) & !(d_dir_f * hat_alpha >= 0.0);
                k += keep as usize;
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
            let mut stopper: Option<Cand> = None;
            for c in kept {
                if width_inf[c.j] && stopper.map_or(true, |s| *c < s) && !(ban_active && discard_banned_cols.contains(&c.j)) {
                    stopper = Some(*c);
                }
            }
            candidates.extend(kept.iter().filter(|c| stopper.map_or(true, |s| **c <= s) && !(ban_active && discard_banned_cols.contains(&c.j))));
        });
        if profile_work {
            prof_phases::STAT_CANDS.fetch_add(candidates.len(), std::sync::atomic::Ordering::Relaxed);
        }
        if candidates.is_empty() {
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
                    lu = refactorize(std, &basis_pos, Some(&lu))?;
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
        let n_candidates = candidates.len();
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
                    lu = refactorize(std, &basis_pos, Some(&lu))?;
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
                    density_bfrt.record(slope_nnz, m);
                    combined_nnz = slope_nnz;
                } else if lu.should_use_dense_solve_tracked(combined_touched.len(), &density_bfrt) {
                    // 傾きチャネル(まれ)はここで単独に解く。基底チャネルは入る列の融合 FTRAN に
                    // 相乗りする (`combined_deferred`) か、ここで解く。密度サンプルはどちらでも
                    // (基底, 傾き) の順に記録する。
                    let slope_nnz = if slope_nonzero {
                        lu.solve_into(&combined_slope, &mut lu_scratch, &mut combined_alpha_slope)
                    } else {
                        lu.add_zero_rhs_solve_ticks(false);
                        0
                    };
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
                    let base_nnz = lu.solve_sparse_into(&sparse_base_buf, &mut sparse_scratch, &mut gp_scratch, &mut combined_alpha_base);
                    let slope_nnz = if slope_nonzero {
                        sparse_slope_buf.extend(combined_touched.iter().map(|&i| (i, combined_slope[i])));
                        lu.solve_sparse_into(&sparse_slope_buf, &mut sparse_scratch, &mut gp_scratch, &mut combined_alpha_slope)
                    } else {
                        lu.add_zero_rhs_solve_ticks(true);
                        0
                    };
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
                    let (a_nnz, b_nnz, c_nnz) = lu.solve_into_triple_capture(
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
                    );
                    combined_base_nnz = c_nnz;
                    tau_ready = true;
                    tau_nnz = Some(b_nnz);
                    a_nnz
                } else if fused_dse_ftran {
                    // DSE の `tau = B^-1 rho_p` FTRAN を同じ走査に融合する
                    // (`solve_into_pair_capture` 参照)。
                    let (a_nnz, b_nnz) = lu.solve_into_pair_capture(&dense_q, &rho, &mut lu_scratch, &mut tau_scratch, &mut alpha_full, &mut tau, &mut a_tilde_buf, Some(&mut rho_steps));
                    tau_ready = true;
                    tau_nnz = Some(b_nnz);
                    a_nnz
                } else {
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
                    let (a_nnz, b_nnz, c_nnz) = lu.solve_sparse_into_triple_capture(
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
                    );
                    combined_base_nnz = c_nnz;
                    tau_ready = true;
                    tau_nnz = Some(b_nnz);
                    a_nnz
                } else if fused_dse_ftran {
                    let (a_nnz, b_nnz) = lu.solve_sparse_into_pair_capture(
                        std.cols.col(q),
                        &rho,
                        &mut sparse_scratch,
                        &mut gp_scratch,
                        &mut tau_scratch,
                        &mut alpha_full,
                        &mut tau,
                        &mut a_tilde_buf,
                        Some(&mut rho_steps),
                    );
                    tau_ready = true;
                    tau_nnz = Some(b_nnz);
                    a_nnz
                } else {
                    lu.solve_sparse_into_capture(std.cols.col(q), &mut sparse_scratch, &mut gp_scratch, &mut alpha_full, &mut a_tilde_buf)
                };
                gp_scratch.u_hyper = false;
                alpha_nnz = result_nnz;
                density_col_aq.record(result_nnz, m);
            }
            if let Some(n) = tau_nnz {
                density_tau.record(n, m);
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
                lu = refactorize(std, &basis_pos, Some(&lu))?;
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
                Some(if combined_pending && combined_slope_nonzero {
                    compact_rows(m, &mut xb_rows, |i| (alpha_full[i].to_bits() | combined_alpha_base[i].to_bits() | combined_alpha_slope[i].to_bits()) << 1)
                } else if combined_pending {
                    compact_rows(m, &mut xb_rows, |i| (alpha_full[i].to_bits() | combined_alpha_base[i].to_bits()) << 1)
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

        timed!(profile_phases, prof_phases::DSE_UPDATE, {
            if !tau_ready {
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
        if shortlist_ready {
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
        if made_infeasible_progress || made_m_side_progress {
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

        // 双対値の増分更新(Huangfu & Hall §2.2.3): PRICE が触った全列で
        // `d[j] -= theta_d * a_p[j]`。BFRT フリップは `B`/`c_B` を変えないので `d` に影響しない。
        let theta_d = dj_q / alpha_q;
        if debug_d_drift_ext && theta_d.abs() > 1e3 {
            eprintln!(
                "DEBUG_D_DRIFT: LARGE theta_d at iter={iter_idx}: q={q} dj_q={dj_q} alpha_q={alpha_q} theta_d={theta_d} touched_cols={} r={r}",
                touched_cols.len()
            );
        }
        timed!(profile_phases, prof_phases::DUAL_UPDATE, {
            // 更新と `a_p` のクリアを 1 パスで行う。
            for &j in &touched_cols {
                d[j] -= theta_d * a_p[j];
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            if price_nonbasic_only {
                // HiGHS `HEkkDual::updateDual` と同じく、入る列と離基列の `d` を直接設定する
                // (`price_nonbasic_only` 参照)。
                d[q] = 0.0;
                d[leaving_var] = -theta_d;
            }
        });

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
            !lu.try_update_precomputed(r, &a_tilde_buf, &e_tilde_buf, tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64))
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
        if !need_refactor && synth_clock_should_refactor(&lu) {
            need_refactor = true;
            if profile_phases {
                prof_phases::REFACTOR_CAUSE_CLOCK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        if !need_refactor && since_check >= tunable!("ENOMOTO_T_XB_CHECK_INTERVAL", XB_CHECK_INTERVAL, usize) {
            since_check = 0;
            let bump_too_big = lu.fill_count() > tunable!("ENOMOTO_T_FT_BUMP_LIMIT_FACTOR", FT_BUMP_LIMIT_FACTOR, usize) * m.max(1);
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
                let sample_clear = xb_drift_sample >= 2 && updates > XB_CHECK_INTERVAL && {
                    let (sb, ss) = sampled_residual_affine(std, &basis_pos, &x_b_base, &x_b_slope, check_rhs_base, check_rhs_slope, phase == Phase::A, xb_drift_sample, drift_sample_offset);
                    drift_sample_offset = (drift_sample_offset + 1) % xb_drift_sample;
                    if env_str!("ENOMOTO_DEBUG_XB_DRIFT_EXT").is_some() {
                        eprintln!("DEBUG_XB_DRIFT_SAMPLE: iter={iter_idx} est_base={sb:.3e} est_slope={ss:.3e} effective_tol={effective_drift_tol:.3e}");
                    }
                    sb.max(ss) <= xb_drift_sample_guard * effective_drift_tol
                };
                if sample_clear {
                    need_refactor = false;
                } else {
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
                    if need_refactor {
                        drift_trigger_count += 1;
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
            if profile_phases {
                prof_phases::REFACTOR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            timed!(profile_phases, prof_phases::REFACTOR, {
                lu = refactorize(std, &basis_pos, Some(&lu))?;
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
    // `None`(`NotSolved`)を返す。
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
    // 摂動済みコスト(`super::perturb_costs`)。この段の双対値 `d` はこれに基づく。
    let active_cost = super::perturb_costs(std);

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
            let mut stall = super::PrimalStallState::new();
            if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
                eprintln!("DEBUG_EXT: polish DUAL->PRIMAL cleanup handoff at polish_iter={iter_idx}");
            }
            // ここでの特異基底は `None`(`NotSolved`)として伝播する。
            let handoff_t0 = std::time::Instant::now();
            let handoff_iters0 = super::prof_phases::RUN_PHASE_ITERS.load(std::sync::atomic::Ordering::Relaxed);
            // `ENOMOTO_HANDOFF_INCREMENTAL=1` (S4、既定オフ): `run_phase` の代わりに
            // `x_B`/`d` 増分維持版の主単体ループを使う(経路は変わりうる)。
            let status = if tunable!("ENOMOTO_HANDOFF_INCREMENTAL", 0u8, u8) != 0 {
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
            return Some(SimplexResult {
                status: status.clone(),
                x: if status == Status::Optimal { Some(t.x[0..t.n_structural()].to_vec()) } else { None },
            });
        };

        // BTRAN: `rho = B^-T e_r`。この反復の `try_update_precomputed` 用に
        // `e_tilde_buf` もキャプチャする。
        lu.solve_transpose_unit_capture(r, &mut lu_scratch, &mut rho, &mut e_tilde_buf);

        // 行方向の疎 PRICE: `rho` の非ゼロ行だけを走査して `a_p = rho^T A` を作る
        // (固定列は除外。基底列は除外しない)。
        for i in 0..m {
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
            lu = refactorize(std, basis_pos, Some(&lu))?;
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
            lu = refactorize(std, basis_pos, Some(&lu))?;
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

/// 2 つの同長ベクトルの内積 `Σ a_i b_i`(先頭から順に加算)。
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum()
}

/// 拡張双対単体法の単体テスト(手作りの小さな `StdForm` で各経路を検証する)。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::{CscMat, CsrMat};
    use std::sync::atomic::Ordering::Relaxed;

    /// 各行がスラック項を含んだ疎行リストから `StdForm` を直接組み立てる
    /// (presolve を経由せず、[`solve_lp_dual_extended`] 単体を検証するため)。
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
        let res = solve_lp_dual_extended(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
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
        let res = solve_lp_dual_extended(&std, &crate::types::LpOptions::default()).expect("cleanup case (A) parks the free survivor at Zero instead of bailing");
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
        let res = solve_lp_dual_extended(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
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
        let res = solve_lp_dual_extended(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
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
        let res = solve_lp_dual_extended(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Optimal);
        assert!(approx(res.x.unwrap()[0], 0.0));
    }

    /// 行が無い問題で、コストが無限側を向く列は非有界と報告される。
    #[test]
    fn m_zero_unbounded_direction_reports_unbounded() {
        // 行なし。x0 ∈ [0, +inf)、コストは無限側を好む(`m == 0` 近道の非有界分岐)。
        let std = std_form(&[], vec![], vec![-1.0], vec![0.0], vec![f64::INFINITY]);
        let res = solve_lp_dual_extended(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Unbounded);
    }

    /// 非有界列の有無にかかわらず、実行不能な行は `Infeasible` と報告される。
    #[test]
    fn infeasible_row_is_reported_regardless_of_an_unbounded_column() {
        // x0 ∈ [0,+inf)、x1 ∈ [0,5]、x0 + x1 + s = -1、s ∈ [0,0]。
        // x0, x1 >= 0 の和が負になることはないので実行不能。
        let std = std_form(&[vec![(0, 1.0), (1, 1.0), (2, 1.0)]], vec![-1.0], vec![0.0, 0.0, 0.0], vec![0.0, 0.0, 0.0], vec![f64::INFINITY, 5.0, 0.0]);
        let res = solve_lp_dual_extended(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
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
        let res = solve_lp_dual_extended(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
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
        let res = solve_lp_dual_extended(&std, &crate::types::LpOptions::default()).expect("should reach a mathematical answer, not the None fallback sentinel");
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
}
