//! ソルバー全体の閾値・許容誤差・反復上限などのパラメータを一か所にまとめたもの。
//!
//! 各モジュールは `use crate::params::<領域>::{...}` で必要な定数を取り込む。
//! 一部の値は `tunable!("ENOMOTO_T_...", 定数, 型)` 経由で環境変数から上書きでき、
//! ここの値はその既定値になる (再ビルドなしで A/B 比較するため)。
//! 値の決め方・試して却下した値などの経緯は `docs/improvement_history.md` を参照。

/// 単体法共通 (src/simplex.rs)
pub(crate) mod simplex {
    /// 汎用のゼロ判定許容誤差 (係数・比率・被約費用が実質 0 かどうか)。
    pub(crate) const TOL: f64 = 1e-9;

    /// `max_iters_for` が返す反復上限の下限値。
    pub(crate) const MAX_ITERS_FLOOR: usize = 20_000;

    /// `max_iters_for` が返す反復上限の絶対的な上限値 (病的な問題での長時間化を防ぐ保険)。
    pub(crate) const MAX_ITERS_CEILING: usize = 2_000_000;

    /// 反復上限の倍率: 上限 = `MAX_ITERS_SCALE * (m + n_total)` (を下限/上限で挟む)。
    pub(crate) const MAX_ITERS_SCALE: usize = 20;

    /// chuzr が基底変数の上下限違反とみなす主実行可能性許容誤差 (HiGHS の
    /// `primal_feasibility_tolerance` と同じ 1e-7)。`TOL` より緩い。
    pub(crate) const PRIMAL_FEAS_TOL: f64 = 1e-7;

    /// 双対法 BFRT の Harris 2 パス比率テストの許容幅。比率が少し小さい候補でも
    /// ピボットの条件が良ければ選べるよう、受け入れ窓を後ろ向きに広げる量。
    pub(crate) const HARRIS_RATIO_TOL: f64 = 1e-7;

    /// 主単体法 `run_phase` の比率テスト第 2 パスの受け入れ窓を広げる量
    /// ([`HARRIS_RATIO_TOL`] の主単体法版)。ごく小さいピボットを避けるため。
    pub(crate) const PRIMAL_HARRIS_TOL: f64 = 1e-7;

    /// ピボットの目的関数への寄与 (`theta * d_q`) がこれ未満なら「進展なし」として
    /// 停滞カウンタ (`bland_mode` 判定) を進める。
    pub(crate) const STALL_PROGRESS_EPS: f64 = 1e-9;

    /// 再分解トリガ (2): FT 更新後のピボットがこれより小さければ更新を拒否して再分解する。
    /// 主単体法の比率テストでブロック行とみなす `|alpha_i|` の下限にも使う。
    pub(crate) const FT_MIN_PIVOT: f64 = 1e-7;

    /// 再分解トリガ (1)(3) を検査する周期 (反復数)。
    pub(crate) const FT_CHECK_INTERVAL: usize = 5;

    /// トリガ (1) の残差検査は `FT_CHECK_INTERVAL` 周期の検査のうちこの回数に 1 回だけ行う
    /// (= `FT_CHECK_INTERVAL * RESIDUAL_CHECK_MULTIPLIER` 反復ごと)。
    pub(crate) const RESIDUAL_CHECK_MULTIPLIER: usize = 20;

    /// 再分解トリガ (1): 真の基底残差 `‖A_B x_B - rhs‖` がこれを超えたら再分解する。
    pub(crate) const FT_RESIDUAL_TOL: f64 = 1e-4;

    /// 再分解トリガ (3): eta ファイルの累積 fill が `FT_BUMP_LIMIT_FACTOR * m` を超えたら再分解する。
    pub(crate) const FT_BUMP_LIMIT_FACTOR: usize = 64;

    /// 再分解トリガ (4): FT 更新回数がこれを超えたら無条件に再分解する。
    pub(crate) const FT_MAX_UPDATES: usize = 300;

    /// 再分解トリガ (5) (HiGHS `updateVerify` 相当): ピボット要素の行方向の値 (PRICE) と
    /// 列方向の値 (FTRAN) の相対差がこれを超えたら、コミット前に再分解する。
    pub(crate) const UPDATE_VERIFY_TOL: f64 = 1e-7;

    /// EXPAND 巡回回避 (Gill, Murray, Saunders & Wright 1989) の基準実行可能性許容誤差
    /// `delta_f`。作業許容誤差 `delta` は常にこれ未満に保つ。非基底変数を上下限に
    /// 戻すときのスナップ幅にも使う (§4.2-4.3)。
    pub(crate) const EXPAND_DELTA_F: f64 = 1e-6;

    /// EXPAND の 1 拡大系列の反復数 (この反復数ごとにリセットする, §4.2)。
    pub(crate) const EXPAND_K: usize = 50;

    /// 拡大系列の開始時の作業許容誤差 (§4.2: `delta_0 = 0.5 delta_f`)。
    pub(crate) const EXPAND_DELTA_0: f64 = 0.5 * EXPAND_DELTA_F;

    /// `EXPAND_K` 反復で近づく作業許容誤差の上限 (§4.2: `delta_K = 0.99 delta_f`)。
    pub(crate) const EXPAND_DELTA_K: f64 = 0.99 * EXPAND_DELTA_F;

    /// 作業許容誤差の 1 反復あたりの増分 (§4.2: `tau = (delta_K - delta_0) / K`)。
    /// 比率テストの最小ステップ `tau / |pivot|` にも使う。
    pub(crate) const EXPAND_TAU: f64 = (EXPAND_DELTA_K - EXPAND_DELTA_0) / (EXPAND_K as f64);

    /// 最急辺/DSE 重みの下限 (丸め誤差で極小・負になった重みを防ぐ)。
    pub(crate) const STEEPEST_EDGE_FLOOR: f64 = 1e-10;

    /// 行数/要素数がこれを超えたら rayon 並列版を使う (chuzr の行走査、DSE 重み更新、
    /// スケーリングの列ノルム計算)。現実的な問題サイズでは常に逐次になる安全弁的な値。
    pub(crate) const RAYON_SIZE_THRESHOLD: usize = 100_000;

    /// 全列数 (構造 + スラック) がこれ以上なら部分価格付けを使う (主単体法の入る変数選択と
    /// 双対法の chuzc1)。
    pub(crate) const PARTIAL_PRICING_THRESHOLD: usize = 300;

    /// 部分価格付けのグループ数: まず約 `1/PARTIAL_PRICING_GROUPS` の列を標本として調べる。
    pub(crate) const PARTIAL_PRICING_GROUPS: u64 = 10;

    /// 前処理での Ruiz スケーリングの反復回数。
    pub(crate) const RUIZ_ITERS: usize = 10;

    /// 前処理 1 ラウンドあたりの制約伝播 (上下限の強化) のパス数。
    pub(crate) const PROPAGATION_PASSES: usize = 2;

    /// `presolve::run_extended` の外側ラウンド (propagate → dualfix → rowsingleton →
    /// doubleton → colsingleton) の最大回数。何も変化しなくなれば早く止まる。
    pub(crate) const PRESOLVE_ROUNDS: usize = 20;

    /// 外側ラウンド 1 回あたりの rowsingleton ⇔ colsingleton の内側反復の最大回数。
    pub(crate) const ROWSINGLETON_COLSINGLETON_INNER_ROUNDS: usize = 1;

    /// 連結成分分解で、いずれかの成分の変数数がこれ以上なら成分を rayon で並列に解く。
    pub(crate) const PARALLEL_COMPONENT_MIN_VARS: usize = 200;

    /// 主単体法の停滞上限 `stall_limit = max(PRIMAL_STALL_LIMIT_PER_ROW * m, PRIMAL_STALL_LIMIT_MIN)`
    /// の行数あたりの係数。進展のないピボットがこれを超えて続いたら Bland 規則に切り替える。
    pub(crate) const PRIMAL_STALL_LIMIT_PER_ROW: usize = 5;

    /// 主単体法の停滞上限 `stall_limit` の最小値。
    pub(crate) const PRIMAL_STALL_LIMIT_MIN: usize = 500;

    /// 費用摂動: 最大費用の絶対値がこれを超えたら 4 乗根で減衰させる (HiGHS と同じ)。
    pub(crate) const COST_PERTURB_LARGE_COST: f64 = 100.0;

    /// 費用摂動: 箱型 (両側有限) 列の割合がこれ未満なら、最大費用を
    /// [`COST_PERTURB_FEW_BOXED_COST_CAP`] で頭打ちにする。
    pub(crate) const COST_PERTURB_BOXED_FRACTION: f64 = 0.01;

    /// 費用摂動: 箱型列がごく少ないときの最大費用の頭打ち値。
    pub(crate) const COST_PERTURB_FEW_BOXED_COST_CAP: f64 = 1.0;

    /// 費用摂動の基準係数: 基準の大きさ = `COST_PERTURB_BASE * (減衰後の最大費用)`。
    pub(crate) const COST_PERTURB_BASE: f64 = 5e-7;
}

/// 拡張双対単体法 (src/simplex/extended_dual.rs)
pub(crate) mod extended_dual {
    /// `x_B(M)` の増分維持値のドリフト検査(と eta フィル検査)を行う主ループの反復間隔。
    /// `fill_count` にかかわらず毎回検査し、古典法のように `RESIDUAL_CHECK_MULTIPLIER` で
    /// さらに間引くことはしない(比較の多くが傾き項で `LEX_REL_TOL = 1e-9` という厳しさで決まるため)。
    /// 値は `simplex::FT_CHECK_INTERVAL`(5)と同じ。
    pub(crate) const XB_CHECK_INTERVAL: usize = super::simplex::FT_CHECK_INTERVAL;

    /// `x_B(M)` のドリフト検査の許容誤差(残差 `‖A_B x_B - rhs‖` の絶対値、基底・傾きの両チャネル)。
    /// 比較が `1e-9` の相対許容誤差で決まるので、古典法の `FT_RESIDUAL_TOL`(1e-4)より十分厳しくする。
    /// 1 回の求解内で、ドリフト起因の再分解が [`XB_DRIFT_ESCALATION_STEP`] 回起きるごとに
    /// [`XB_DRIFT_ESCALATION_FACTOR`] 倍に緩める(上限 [`XB_DRIFT_TOL_MAX`])。再分解が少ない求解は
    /// この値のまま。`ENOMOTO_XB_DRIFT_TOL` で上書き可。
    pub(crate) const XB_DRIFT_TOL: f64 = 1e-8;

    /// 同じ求解内でドリフト起因の再分解がこの回数起きるごとに、[`XB_DRIFT_TOL`] の実効値を
    /// [`XB_DRIFT_ESCALATION_FACTOR`] 倍にする(上限 [`XB_DRIFT_TOL_MAX`])。
    pub(crate) const XB_DRIFT_ESCALATION_STEP: usize = 10;

    /// [`XB_DRIFT_ESCALATION_STEP`] 回ごとに [`XB_DRIFT_TOL`] の実効値に掛ける倍率。
    pub(crate) const XB_DRIFT_ESCALATION_FACTOR: f64 = 10.0;

    /// 段階的に緩めた [`XB_DRIFT_TOL`] の上限(古典法の `FT_RESIDUAL_TOL`、1e-4)。
    pub(crate) const XB_DRIFT_TOL_MAX: f64 = super::simplex::FT_RESIDUAL_TOL;

    /// 数値的原因による再分解(FT 更新の拒否、`x_B(M)` ドリフト、`d` ドリフト、PRICE と大きく
    /// 食い違うピボット)がこの回数起きるごとに、LU のピボット閾値を 1 段引き上げる
    /// (`sparse_lu::escalate_pivot_threshold`)。コスト起因のトリガ(eta フィル、更新回数上限、
    /// 合成クロック)は数えない。**0 = 無効(既定)**。`ENOMOTO_PIVOT_ESCALATION_STEP` で上書き可。
    pub(crate) const PIVOT_ESCALATION_STEP: usize = 0;

    /// トリガ (4): FT 更新回数の上限を `m` に比例させる係数。上限は
    /// `max(FT_MAX_UPDATES_FACTOR * m, FT_MAX_UPDATES_FLOOR)`(`ft_max_updates`)。eta 連鎖の
    /// 無制限な伸長を防ぐめったに発火しない安全網。`ENOMOTO_T_FT_MAX_UPDATES_FACTOR` で上書き可。
    pub(crate) const FT_MAX_UPDATES_FACTOR: f64 = 3.0;

    /// トリガ (4) の上限の下限(古典法の `simplex::FT_MAX_UPDATES`、300)。`m` が小さくてもこれより弱くしない。
    pub(crate) const FT_MAX_UPDATES_FLOOR: usize = super::simplex::FT_MAX_UPDATES;

    /// トリガ (5): 決定的な「合成クロック」による再分解。FT 更新回数が
    /// [`SYNTH_CLOCK_MIN_UPDATES`] 以上で、前回の分解以降の求解側の演算量推定(`synth_tick`)が
    /// `SYNTH_CLOCK_FACTOR * build_tick`(分解自体の演算量推定)に達したら再分解する
    /// (HiGHS `HEkk::updateFactor` と同じ考え方。壁時計ではなく演算回数なので再現性がある)。
    /// `ENOMOTO_SYNTH_CLOCK_FACTOR` で上書き可。
    pub(crate) const SYNTH_CLOCK_FACTOR: f64 = 16.0;

    /// トリガ (5) が発火するのに必要な最小の FT 更新回数(HiGHS の
    /// `kSyntheticTickReinversionMinUpdateCount` と同じ 50)。更新直後の誤発火を防ぐ。
    pub(crate) const SYNTH_CLOCK_MIN_UPDATES: usize = 50;

    /// 増分維持している被約費用 `d` のドリフト検査の相対許容誤差: `‖d - fresh_d‖`(固定列を除く)が
    /// `D_DRIFT_TOL * max(‖fresh_d‖, 1)` を超えたら再分解する。
    pub(crate) const D_DRIFT_TOL: f64 = 1.0;

    /// PRICE によるピボット要素 `alpha_q` と FTRAN による `alpha_full[r]` の相対差がこれを超えたら
    /// 「桁違いの不一致」(`pivot_grossly_inconsistent`)としてピボットを破棄する。
    /// `update_verify` の厳しい許容誤差(1e-7)よりずっと緩く、`update_count` によらず常に検査する。
    pub(crate) const D_GROSS_MISMATCH_REL_TOL: f64 = 0.5;

    /// `x_B(M)` の `M` 係数は厳密には 0 か 1 のオーダーなので、絶対値がこれ未満の係数は
    /// LU/更新の雑音とみなして 0 に丸める(`snap_slope`)。`ENOMOTO_T_X_B_SLOPE_NOISE` で上書き可。
    pub(crate) const X_B_SLOPE_NOISE: f64 = 1e-7;

    /// `M` 係数の絶対許容誤差: 段階 A → B の移行で、基底の `x^1_j` が傾き境界 `l^1_j`/`u^1_j` 上に
    /// あるとみなす(その境界を `l^B`/`u^B` に残す)範囲。`Affine1::gt_zero` の傾き閾値と同じ値。
    pub(crate) const SLOPE_TOL: f64 = 1e-9;

    /// 最適値の傾き `z^1`(`z(M) = z^0 + z^1 M`)が `z^1 < 0`(実行不能または非有界、
    /// `prop:trichotomy`)とみなされる負の閾値(`-Z_SLOPE_TOL` 未満)。段階 A の早期終了と
    /// `finish` の非有界判定で共有する。
    pub(crate) const Z_SLOPE_TOL: f64 = 1e-7;

    /// `M`-アフィン量(`Affine1`/`Score2`)の辞書式比較で使う相対許容誤差。各成分の差が
    /// `LEX_REL_TOL * max(|a|, |b|, 1)` 以下なら等しいとみなす(別経路で計算された同じ値の
    /// 数 ULP のずれを吸収する)。`Affine1::gt_zero`/`deviation_flat` の「正」判定、`bfrt_reached` の
    /// 傾き比較、`Score2` の許容誤差の基準値(`ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL` の既定値)、
    /// polish の BFRT 到達判定 `reach_tol` の相対部分にも使う。
    pub(crate) const LEX_REL_TOL: f64 = 1e-9;

    /// 停滞ピボット数の上限 `stall_limit = max(STALL_LIMIT_PER_ROW * m, STALL_LIMIT_MIN)` の
    /// 行数あたりの係数。超えたら Bland 規則(`bland_mode`)に切り替える。
    pub(crate) const STALL_LIMIT_PER_ROW: usize = 5;

    /// `stall_limit` の下限(小さな問題でも最低この回数の停滞は許す)。
    pub(crate) const STALL_LIMIT_MIN: usize = 500;

    /// 「最大改善」chuzr エスカレーションで試行 PRICE を行う DSE 上位候補行の数。
    pub(crate) const GREATEST_IMPROVEMENT_TOP_K: usize = 8;

    /// 「最大改善」エスカレーションの発動閾値 `max(stall_limit / GREATEST_IMPROVEMENT_STALL_DIVISOR,
    /// GREATEST_IMPROVEMENT_STALL_MIN)` の除数(`bland_mode` よりずっと早く発動させる)。
    pub(crate) const GREATEST_IMPROVEMENT_STALL_DIVISOR: usize = 4;

    /// 「最大改善」エスカレーションの発動閾値の下限。
    pub(crate) const GREATEST_IMPROVEMENT_STALL_MIN: usize = 30;

    /// 実行不能行数プラトー検出の上限 `min(INFEASIBLE_PLATEAU_STALL_MULT * stall_limit,
    /// MAX_ITERS_FLOOR / INFEASIBLE_PLATEAU_BUDGET_DIVISOR)` の `stall_limit` に対する倍率
    /// (健全だが遅い求解の揺らぎで誤発火しないよう大きめにする)。
    pub(crate) const INFEASIBLE_PLATEAU_STALL_MULT: usize = 4;

    /// 実行不能行数プラトー検出の上限を反復予算 `MAX_ITERS_FLOOR` の何分の一に抑えるか
    /// (予算内で確実に発火できるようにする)。
    pub(crate) const INFEASIBLE_PLATEAU_BUDGET_DIVISOR: usize = 4;

    /// S2: ドリフト残差の事前検査で使う巡回行サンプルの間隔 `k`(`k >= 2` で有効、0 = オフ)。
    /// `ENOMOTO_XB_DRIFT_SAMPLE` で上書き可。
    pub(crate) const XB_DRIFT_SAMPLE_K: usize = 0;

    /// S2: サンプル推定の残差が `XB_DRIFT_SAMPLE_GUARD * 許容誤差` 以下なら全体の検査を省く。
    /// `ENOMOTO_XB_DRIFT_SAMPLE_GUARD` で上書き可。
    pub(crate) const XB_DRIFT_SAMPLE_GUARD: f64 = 0.1;

    /// 新規残差の下限の係数(0 = オフ): 再分解直後の残差 `r` がすでに許容誤差の
    /// [`XB_DRIFT_FRESH_FLOOR_FRAC`] 倍を超えていれば、次の再分解まで `XB_DRIFT_FRESH_FLOOR_FACTOR * r`
    /// を許容誤差の下限にする。`ENOMOTO_XB_DRIFT_FRESH_FLOOR` で上書き可。
    pub(crate) const XB_DRIFT_FRESH_FLOOR_FACTOR: f64 = 0.0;

    /// 新規残差の下限を有効にする、許容誤差に対する新規残差の割合。
    /// `ENOMOTO_XB_DRIFT_FRESH_FLOOR_FRAC` で上書き可。
    pub(crate) const XB_DRIFT_FRESH_FLOOR_FRAC: f64 = 0.5;

    /// 相対下限の係数(0 = オフ): FT 更新が `XB_CHECK_INTERVAL` 回を超えたら、許容誤差を
    /// `XB_DRIFT_REL_K * 再分解直後の残差` 以上にする。`ENOMOTO_XB_DRIFT_REL_K` で上書き可。
    pub(crate) const XB_DRIFT_REL_K: f64 = 0.0;

    /// FT 更新回数がこれ未満の若い eta ファイルでは許容誤差を
    /// [`XB_DRIFT_MIN_UPDATES_MULT`] 倍に緩める(0 = オフ)。`ENOMOTO_XB_DRIFT_MIN_UPDATES` で上書き可。
    pub(crate) const XB_DRIFT_MIN_UPDATES: usize = 0;

    /// [`XB_DRIFT_MIN_UPDATES`] 未満のときの許容誤差の倍率。`ENOMOTO_XB_DRIFT_MIN_UPDATES_MULT` で上書き可。
    pub(crate) const XB_DRIFT_MIN_UPDATES_MULT: f64 = 100.0;

    /// 実験的な `Score2` 適応許容誤差で、M 側の進展が止まったときに許容誤差を基準値へ引き戻す
    /// 速さの半減期(反復数)。`ENOMOTO_SCORE2_STALL_HALFLIFE` で上書き可。
    pub(crate) const SCORE2_STALL_HALFLIFE: f64 = 50.0;

    /// 実験的な stuck_row ブースト: 同じ行でピボット破棄がこの回数以上続いたらブーストする。
    /// `ENOMOTO_STUCK_ROW_BOOST_THRESHOLD` で上書き可。
    pub(crate) const STUCK_ROW_BOOST_THRESHOLD: usize = 3;

    /// 実験的な stuck_row ブースト: 該当行の逸脱の `M` 係数に掛ける倍率(1.0 = 無効)。
    /// `ENOMOTO_STUCK_ROW_BOOST_FACTOR` で上書き可。
    pub(crate) const STUCK_ROW_BOOST_FACTOR: f64 = 1.0;

    /// S11(実験的): chuzr 候補短縮リストの長さ `K`(0 = オフ)。`ENOMOTO_T_CHUZR_SHORTLIST` で上書き可。
    pub(crate) const CHUZR_SHORTLIST_K: usize = 0;

    /// S11: 実行不能行プールが `CHUZR_SHORTLIST_MIN_POOL_FACTOR * K` 行を超えるときだけ短縮リストを使う。
    pub(crate) const CHUZR_SHORTLIST_MIN_POOL_FACTOR: usize = 4;

    /// S11: 短縮リストが `CHUZR_SHORTLIST_MAX_LEN_FACTOR * K + CHUZR_SHORTLIST_MAX_LEN_SLACK` 行を
    /// 超えたら無効にして全走査に戻す(係数部分)。
    pub(crate) const CHUZR_SHORTLIST_MAX_LEN_FACTOR: usize = 4;

    /// S11: 短縮リストの最大長の定数部分。
    pub(crate) const CHUZR_SHORTLIST_MAX_LEN_SLACK: usize = 64;

    /// S9(既定オフ): `rho` の非ゼロ数が `PRICE_COLUMN_DENSITY * m` を超えたら列方向 PRICE に切り替える
    /// (HiGHS の切り替え点)。`ENOMOTO_PRICE_COLUMN_DENSITY` で上書き可。
    pub(crate) const PRICE_COLUMN_DENSITY: f64 = 0.1;

    /// S8: 前反復の PRICE 行数が `PRICE_LIST_DENSITY * m` 以下なら `rho` の非ゼロ行一覧を作って
    /// それだけを走査する。`ENOMOTO_T_PRICE_LIST_DENSITY` で上書き可。
    pub(crate) const PRICE_LIST_DENSITY: f64 = 0.1;

    /// S6: `x_B` 更新の非ゼロ行数の推定が `XB_LIST_DENSITY * m` 以下なら非ゼロ行一覧を作って
    /// それだけを走査する。`ENOMOTO_T_XB_LIST_DENSITY` で上書き可。
    pub(crate) const XB_LIST_DENSITY: f64 = 0.3;

    /// C5: 入る列の疎 FTRAN で、結果密度の移動平均がこれ未満なら `U` 段を超疎で解く(0 = オフ)。
    /// `ENOMOTO_FTRAN_U_HYPER` で上書き可。
    pub(crate) const FTRAN_U_HYPER_DENSITY: f64 = 0.1;

    /// C5: 融合 DSE `tau` FTRAN で、結果密度の移動平均がこれ未満なら `U` 段を超疎で解く(0 = オフ)。
    /// `ENOMOTO_FTRAN_U_HYPER_TAU` で上書き可。
    pub(crate) const FTRAN_U_HYPER_TAU_DENSITY: f64 = 0.1;

    /// `pivot_grossly_inconsistent` の相対差の分母の下限(0/0 を避けるためだけの極小値)。
    pub(crate) const GROSS_MISMATCH_SCALE_FLOOR: f64 = 1e-300;

    /// `compact_rows` がまとめて非ゼロ判定する行ブロックの大きさ。
    pub(crate) const COMPACT_ROWS_BLOCK: usize = 8;
}

/// 疎 LU 分解・Forrest-Tomlin 更新 (src/simplex/lu.rs)
pub(crate) mod lu {
    /// Threshold-pivoting stability floor (see this module's own top docs): a
    /// pivot candidate must be at least this fraction of its column's live max
    /// magnitude to be eligible, regardless of Markowitz count. Raised from the
    /// textbook-default `0.1` after measuring that `0.1` lets `factorize()` pick
    /// pivots numerically weak enough to make the *resulting* `L`/`U` drift
    /// faster under `extended_dual`'s `XB_DRIFT_TOL` check (see that constant's
    /// own docs) — i.e. a chain of numerically-marginal Markowitz choices, not
    /// any single one bad enough to fail `FT_MIN_PIVOT` outright, was forcing
    /// extra mid-solve refactorizations well before `FT_BUMP_LIMIT_FACTOR`'s own
    /// eta-fill trigger would have. Netlib's `pilot` (the clearest case)
    /// dropped from 190 drift-triggered refactorizations to 88 at `0.25`
    /// (measured twice, deterministic — refactor counts don't vary run to run,
    /// only wall-clock does), for a ~51% wall-time cut on that instance alone;
    /// `greenbeb`/`fit2p` improved or held flat; `d2q06c` was unchanged within
    /// run-to-run noise (~5%, from system load, confirmed by re-running the
    /// unchanged `0.1` baseline twice). The standard 73-problem Netlib set
    /// (`enomoto_solver.benchmark_highs`, which skips these largest instances on
    /// `n_vars`) is flat within the same noise band either way — this constant
    /// only matters for problems that already refactorize dozens-to-hundreds of
    /// times. `0.5` was tried first and rejected: fill-in from the stricter
    /// floor made every iteration measurably more expensive (`d2q06c`,
    /// `greenbeb`, `fit2p` all ~4% slower net, more than offsetting their own
    /// small refactor-count drops), so `0.5` is *not* simply "more of the same
    /// good direction" — `0.25` is a measured sweet spot, not a floor to keep
    /// pushing from without re-benchmarking.
    /// Since `docs/lu_comparison_enomoto_vs_highs.md` §2.4 this is the
    /// *starting* value of a per-solve threshold that a simplex loop may
    /// escalate ([`pivot_threshold`]) — but that escalation is off by default
    /// (`extended_dual::PIVOT_ESCALATION_STEP`, which records what enabling it
    /// measured), so this remains the floor every solve actually runs at, and
    /// everything measured above still describes the default build.
    pub(crate) const STABILITY: f64 = 0.25;

    /// Ceiling on the escalated pivot threshold ([`escalate_pivot_threshold`])
    /// — HiGHS's own `kMaxPivotThreshold`. [`STABILITY`]'s docs record that a
    /// *static* `0.5` costs ~4% on `d2q06c`/`greenbeb`/`fit2p` through extra
    /// fill-in, which is exactly why this value is reachable only after the
    /// escalation ladder below has evidence that *this* solve is paying more
    /// for instability than it would for fill.
    pub(crate) const PIVOT_THRESHOLD_MAX: f64 = 0.5;

    /// Floor for an operator-supplied `ENOMOTO_PIVOT_THRESHOLD` — HiGHS's own
    /// `kMinPivotThreshold`. Nothing escalates *downwards*, so this only ever
    /// clamps the env override.
    pub(crate) const PIVOT_THRESHOLD_MIN: f64 = 8e-4;

    /// Multiplier applied per [`escalate_pivot_threshold`] step. HiGHS uses
    /// `kPivotThresholdChangeFactor = 5.0` from a `0.1` default; from this
    /// crate's `0.25` a factor of `2.0` lands exactly on
    /// [`PIVOT_THRESHOLD_MAX`] in one step, so the ladder here is
    /// `0.25 -> 0.5`, and a second escalation is a no-op.
    pub(crate) const PIVOT_THRESHOLD_FACTOR: f64 = 2.0;

    /// A column whose *initial* (pre-elimination) degree exceeds this fraction
    /// of `m` is treated as "dense" by `find_best_pivot`'s dense-avoidance
    /// pass — see `MarkowitzState::initially_dense`'s own docs for why a
    /// column's *current* (post-elimination) degree is the wrong thing to
    /// threshold on here. `0.5` catches the handful of near-fully-dense
    /// "trend"/regression columns Netlib `fit1p`/`fit1d`-shaped problems are
    /// built around (confirmed: `fit1p`'s basis has columns with degree
    /// 610-627 out of `m=627`, against a median column degree of `1`) without
    /// also catching moderately-populated columns that pose no real fill-in
    /// risk.
    pub(crate) const DENSE_COL_FRACTION: f64 = 0.5;

    /// How many candidate columns a single `find_best_pivot` call may examine
    /// before it settles for the best pivot it has already found — HiGHS's
    /// `searchLimit = min(nwork, 8)` in `HFactor::buildKernel`
    /// (`docs/lu_comparison_enomoto_vs_highs.md` §2.5), adapted to this file's
    /// bucket scan.
    ///
    /// The existing per-degree-level early exit (`best_score <= deg_col *
    /// deg_col`, the analogue of HiGHS's `merit_limit`) only ever fires at a
    /// *level* boundary, so a single heavily-populated bucket is scanned to
    /// its end no matter how good the pivot found in its first few columns
    /// was. That is the search-explosion case this bound closes: on an
    /// ill-conditioned or fill-heavy step, the low-degree buckets hold
    /// hundreds of columns whose rows all get walked (and whose
    /// `ensure_col_max_abs` recomputes all get paid) to improve on a pivot
    /// that was already acceptable.
    ///
    /// Like HiGHS's, the bound is only honoured once a pivot *has* been found
    /// — `find_best_pivot` never returns `None` because of it, so `factorize`'s
    /// `skip_dense` fallback and its genuine-singularity detection are
    /// unchanged. What it does change is *which* acceptable pivot is returned:
    /// the Markowitz count can be worse than the unbounded scan's, so this
    /// trades (bounded) extra fill-in for a bounded search.
    ///
    /// **`256`, not HiGHS's `8` — measured, see
    /// `analysis/pivot_search_limit_20260922_143000.md`.** `8` was tried first
    /// and rejected: it is not "more of the same good direction", it is a
    /// different intervention. At `8` the bound fires on ordinary steps and
    /// changes the chosen pivot on **64 of the 93** Netlib problems; each such
    /// change perturbs the factorization's last digits, which moves the dual
    /// ratio test's tie-breaks, which moves the iteration count by an amount
    /// whose *sign is effectively arbitrary per problem* (`greenbeb` +18%
    /// iterations, `25fv47` −11%). Reproduced over two independent 93-problem
    /// runs, `8` left three problems past +10% (`greenbeb` +21/+22%, `pilot`
    /// +15/+18%, `grow22` +13/+13%) even though it cut the search everywhere,
    /// and a sweep showed no smaller constant escapes the lottery: `16` made
    /// `pilot87` **2.9x slower**, `64` still perturbed 26 problems.
    ///
    /// `256` is chosen so the bound is a worst-case guard and nothing else. It
    /// fires on 7 of 93 problems, and only one of those (`dfl001`, the single
    /// instance where the unbounded scan is genuinely expensive: 2.96s of a
    /// 22.0s solve, averaging 261 candidate columns per elimination step)
    /// changes materially — its scan drops to 1.54s. The other 86 problems are
    /// bit-identical to the unbounded scan, iteration count and
    /// refactorization count included, so the change cannot regress them at
    /// all. Two independent 93-problem runs: −2.1% and −0.8% in total, no
    /// problem past ±10% in either. `512` was also measured (−3.2%/−, perturbs
    /// only 2 problems) but put `wood1p` at +10.6%, so it fails the same rule
    /// `8` does.
    pub(crate) const PIVOT_SEARCH_LIMIT: usize = 256;

    /// Rows shorter than this are searched for a column linearly rather than
    /// by binary search ([`KernelMatrix::row_get`]). Markowitz elimination is
    /// specifically choosing pivots to keep the active rows short, so the
    /// linear branch is the common one: a run this size fits in one or two
    /// cache lines and scans branch-predictably, where `binary_search` pays a
    /// mispredict per level for the same work.
    pub(crate) const KERNEL_LINEAR_SCAN_MAX: usize = 16;

    /// C5 hyper-sparse `U` stage: give up (and take the plain full scan) once
    /// the DFS has reached more than this fraction of the `m` slots — past it,
    /// sorting the reach and scattering the result by list stop paying for
    /// themselves (HiGHS's own `kHyperFtranU` is `0.10`).
    pub(crate) const U_HYPER_ABORT_FRACTION: f64 = 0.25;

    /// Factorizes the `m x m` sparse matrix given as sparse rows
    /// `(col, value)`. Returns `None` if the matrix is (numerically)
    /// singular — no acceptable pivot remains at some step.
    ///
    /// **A Dulmage-Mendelsohn block-triangularized variant of this function
    /// was implemented, thoroughly validated, and measured — then reverted**:
    /// rows were partitioned into strongly-connected blocks (via bipartite
    /// matching + Tarjan SCC, [`crate::graph::dulmage_mendelsohn_blocks_topological`],
    /// which remains implemented and tested for a possible future, more
    /// targeted revisit) in topological order, each factorized independently,
    /// then reassembled via the block-LU identity `U_ij = L_i^{-1} A_ij` for
    /// "spillover" entries outside a block's own matched columns (`L` itself
    /// stays exactly block-diagonal). Implementation correctness was
    /// confirmed via unit tests (including one that caught a real bug: an
    /// initial version copied spillover entries unchanged, which is only
    /// valid when the emitting block's own `L` is trivial/identity — true for
    /// singleton blocks, which is why singleton-only spillover tests passed
    /// by coincidence before the fix) and zero objective mismatches across
    /// the full 73-problem Netlib benchmark.
    ///
    /// **But it measured as a net ~4% aggregate regression** in a controlled
    /// back-to-back A/B (same machine, same run, only the feature toggled):
    /// dramatic wins on a few instances with genuine block-angular structure
    /// (`fit1p` -44%, `wood1p` -17%, `scsd8` -10%, `sierra`/`sctap3`/`scrs8`
    /// a few percent) were outweighed by a broad ~10-25% tax on most other
    /// medium/large instances (`grow15` +25%, `bnl1` +18%, `modszk1` +17%,
    /// `perold` +16%, `25fv47` +16%, `stocfor2` +14%, `pilotnov` +13%,
    /// `ganges` +11%) — paying bipartite-matching-plus-SCC cost on *every*
    /// refactorization, whether or not it finds anything worth exploiting.
    /// Two cheap pre-gating heuristics were tried to avoid paying that cost
    /// on instances unlikely to benefit, and both failed: (1) whether
    /// `presolve::redundancy`'s own equality-row block decomposition found
    /// structure — `ganges` decomposes beautifully there (1053 blocks, a 1%
    /// bump) yet was still a net loss here, since the *basis* matrix (all
    /// rows, reshuffled by every pivot) doesn't share the *equality
    /// system*'s (static, presolve-time-only) structure; (2) the *basis*
    /// matrix's own bump size at the first real refactorization — `stocfor2`
    /// and `ganges` again showed excellent bump ratios (0.2-1.3%, as good as
    /// or better than the actual winners) yet remained net losses, showing
    /// the fixed decomposition cost itself, not just a poor decomposition
    /// outcome, was the problem. This mirrors HiGHS's own architecture:
    /// `HFactor::buildSimple()` peels off trivial (degree-1/logical) pivots
    /// via a cheap `O(nnz)` sweep with no bipartite matching at all, leaving
    /// full Markowitz elimination (`buildKernel()`) for only the remaining
    /// kernel — this file's own bucket-based `find_best_pivot` already gets
    /// that same cheap benefit for free (confirmed earlier via
    /// `PROF_TOTAL_STEPS`/`PROF_TRIVIAL_STEPS` showing 90-100% of pivots
    /// already resolve trivially), so the *additional*, much more expensive
    /// structure genuine Dulmage-Mendelsohn decomposition can find beyond
    /// that cheap peeling isn't reliably worth its own cost. Fully reverted;
    /// see the project history around this doc comment's own commit for the
    /// full numbers if revisiting.
    /// Input whose nonzero density exceeds this fraction of `m^2` skips
    /// Markowitz elimination entirely in favor of [`factorize_dense_faer`]'s
    /// dense partial-pivoting LU (via the `faer` crate). Markowitz's whole
    /// point is to *minimize fill-in*; a matrix already this dense has none
    /// left to save, so its bucket/degree bookkeeping ([`KernelMatrix`]'s own
    /// row/column runs plus `col_buckets`/`row_buckets`) is pure overhead at
    /// that point — confirmed on a synthetic dense LP
    /// (Netlib has none dense enough to exercise this at all): `factorize`
    /// dominated wall time (95-98%, repeated every few dozen `try_update`
    /// calls since a dense basis's eta fill crosses `FT_BUMP_LIMIT_FACTOR *
    /// m` almost immediately) while this file's own FTRAN-side dense
    /// optimizations (`HybridVec`'s dense arm, `FtLu::should_use_dense_solve`)
    /// together accounted for under 1% of the same wall time — i.e. the eta
    /// chain was never the bottleneck for a dense basis, the cold
    /// factorization was. `0.25` is a first-pass threshold, not yet tuned
    /// against a real dense-problem benchmark (Netlib has none).
    pub(crate) const DENSE_INPUT_FRACTION: f64 = 0.25;

    /// **A second, "peel trivial pivots then Dulmage-Mendelsohn-decompose only
    /// the remaining kernel" variant of block triangularization was also
    /// implemented, tested, and measured — then reverted.** This directly
    /// followed up the first attempt documented below, on the hypothesis that
    /// peeling first (mirroring HiGHS's own `buildSimple()`/`buildKernel()`
    /// split) would fix that attempt's "pays matching+SCC cost on every
    /// refactorization regardless of payoff" problem by shrinking the kernel
    /// matching+SCC actually runs on. It did not: full 73-problem Netlib A/B
    /// showed a **net ~37% aggregate regression** — far worse than the first
    /// attempt's ~4%, and a regression on `fit1p` specifically (+80%), the
    /// exact instance this was meant to speed up. Root cause, confirmed by
    /// direct instrumentation: `fit1p`'s kernel (post-peel) is a single
    /// irreducible ~20-row SCC block every time, so the decomposition gate
    /// *always* rejects it and falls back to a from-scratch
    /// `factorize_flat_markowitz` call — meaning the (redundant) peel work is
    /// paid twice, for zero benefit, every refactorization. Worse, the
    /// underlying premise turned out wrong: `fit1p`'s real cost was never a
    /// large interleaved non-trivial block in the first place. `eliminate`'s
    /// cost is `O(col_rows[pj].len())` (the pivot *column*'s remaining active
    /// rows) times the pivot row's own snapshot size — a pivot with Markowitz
    /// score exactly `0` (row degree `1`, the "trivial" case `PROF_TRIVIAL_STEPS`
    /// counts) is only free when its *column*'s degree is also small; a
    /// degree-1 *row* whose sole entry sits in an otherwise-still-dense
    /// "hub" column is scored as trivial yet costs `O(hub column's current
    /// degree)` to eliminate (every other row sharing that column must be
    /// updated). `fit1p`'s basis apparently has exactly this shape — many
    /// row-degree-1 pivots landing on a handful of not-yet-thinned dense
    /// columns — which no SCC/block decomposition addresses, since those rows
    /// don't form a separable block with the hub column at all. Fully
    /// reverted (including the two dedicated unit tests that validated its
    /// spillover-reassembly correctness, which was never in question — the
    /// numerics were right, just not worth what they cost). See the project
    /// history around this comment's own commit for the full A/B numbers and
    /// the `ENOMOTO_DEBUG_BLOCK_TRIANGULAR` trace output that pinned down the
    /// root cause, if revisiting; `debug_print_block_sizes`
    /// (`ENOMOTO_DEBUG_BLOCK_SIZES`) and the `ENOMOTO_DEBUG_ELIMINATE_COST`
    /// timer below remain as live diagnostics either attempt's numbers came
    /// from.
    /// `factorize`'s own gate for attempting [`factorize_bordered`] before
    /// falling back to plain [`factorize_flat_markowitz`] — see
    /// `factorize_bordered`'s own docs for the technique and why it exists.
    ///
    /// This gate's own detection cost (`detect_border_columns`, one `O(nnz)`
    /// pass) is cheap enough to run unconditionally: a controlled full-73-
    /// problem Netlib A/B (this gate enabled vs. plain
    /// `factorize_flat_markowitz` always) showed no measurable regression on
    /// any instance once run-to-run subprocess scheduling noise was
    /// controlled for (repeated head-to-head timing, not two independently-
    /// scheduled full-batch runs — several apparent double-digit-percent
    /// "regressions" in the first batch-vs-batch comparison, e.g.
    /// `fffff800`/`scfxm1`/`ganges`, vanished under direct repeated
    /// comparison), while several instances beyond `fit1p` itself improved
    /// substantially (`scrs8` -58%, `ship04s` -57%, `shell` -52%, `maros`
    /// -41%, `fit1p` -26%, plus a handful more in the 20-45% range) — this is
    /// the same `k`-nonzero-columns detection [`DENSE_COL_FRACTION`] already
    /// made cheap for `MarkowitzState::initially_dense`'s own purposes,
    /// evidently common enough across Netlib-shaped LPs (not just the
    /// `fit1p`/`fit2p` "trend column" family) to be worth attempting by
    /// default rather than gating behind an opt-in flag.
    ///
    /// **`BORDER_MAX_FRACTION` (`k / m`) is the real, measured constraint —
    /// not an absolute `k` count.** A synthetic-`fit1p`-shaped sweep
    /// (`border_crossover_sweep*` in this module's own tests, `#[ignore]`d,
    /// rerun via `cargo test --release -- --ignored --nocapture border_`) at
    /// both `m=800` and `m=2000` found `factorize_bordered` beating
    /// whatever `factorize_flat_markowitz` would otherwise pick (plain
    /// Markowitz below `is_dense_input`'s own 25% gate, `factorize_dense_faer`
    /// above it — `factorize_bordered` beats *that* too, up to a point) by
    /// **30x-600x** for `k/m` up to `0.40`, crossing over to a wash somewhere
    /// around `k/m ~= 0.5` and a clear loss by `k/m = 0.6` — at *both* `m`
    /// values, i.e. this is a genuine fraction effect (the `k x k` Schur
    /// complement's own `O(k^3)` dense-factor cost, relative to the `(m-k)`-
    /// sized sparse part it's carved out of), not an absolute-`k` one: `m=800,
    /// k=400` and `m=2000, k=1000` (both `k/m=0.5`) landed at the same
    /// break-even point despite `k` itself differing by 2.5x. `0.4` sits with
    /// real margin below the measured crossover.
    ///
    /// The *previous* version of this gate paired that fraction with a
    /// `BORDER_MAX_COUNT` of `200` on the mistaken assumption that unbounded
    /// `k` needed an absolute backstop the way the reverted Dulmage-Mendelsohn
    /// attempt did — the sweep above disproves that directly (`m=2000, k=800`,
    /// five times over `200`, still won by 603x). `BORDER_MAX_COUNT` here is
    /// now a purely defensive sanity bound, sized so its own `O(k^3)` dense
    /// factor stays well under this crate's stated basis-size envelope ("`m`
    /// in the low thousands", per `GpScratch`'s own docs) rather than
    /// something expected to actually bind — `BORDER_MAX_FRACTION` is doing
    /// the real work.
    pub(crate) const BORDER_MAX_FRACTION: f64 = 0.4;

    pub(crate) const BORDER_MAX_COUNT: usize = 3000;

    /// A reuse is abandoned (falling back to a full Markowitz `factorize`)
    /// once the factors it is producing exceed this multiple of the nonzero
    /// count of the last *full* factorization's own `L`+`U`.
    ///
    /// **`1.25` is measured, not guessed.** Fill a reuse produces is not a
    /// one-off cost — it is paid again by every FTRAN/BTRAN for the whole life
    /// of the resulting factorization — and a generous limit is a net *loss*
    /// even though it accepts more reuses: over a 25-problem in-process A/B
    /// (`analysis/` note for this change, §3) the aggregate against the
    /// feature disabled ran `2.0` +2.7%, `1.1` +0.4%, `1.25` -1.5%, with the
    /// `2.0` arm's worst case `greenbeb` +30%. Too *tight* loses the other
    /// way: `1.0`/`1.05` reject nearly every attempt (the basis genuinely
    /// densifies between refactorizations), so the backoff below stops even
    /// trying and the feature turns into pure overhead.
    ///
    /// The reused order was chosen by Markowitz against a *previous* basis;
    /// the current one differs from it by however many Forrest-Tomlin updates
    /// happened since, so the same order can be numerically fine yet produce
    /// far more fill than a fresh Markowitz run would. Fill produced here is
    /// not a one-off cost: it is paid again by every FTRAN/BTRAN for the whole
    /// life of the resulting factorization, which is exactly the trade this
    /// guard exists to cap. The baseline deliberately tracks the last *full*
    /// factorization rather than the immediately-preceding one (see
    /// [`FtLu::fill_baseline`]), so a long chain of reuses cannot ratchet the
    /// limit upward one small increment at a time; a basis whose fill
    /// genuinely grew simply fails this guard once, gets a fresh Markowitz
    /// factorization, and the new baseline is that one's own.
    pub(crate) const REBUILD_FILL_LIMIT: f64 = 1.25;

    /// Absolute floor on a pivot's magnitude: below this the column has
    /// nothing usable left in the remaining submatrix at all, and the whole
    /// attempt is abandoned rather than dividing by (almost) zero. Far below
    /// `simplex.rs`'s own `FT_MIN_PIVOT` deliberately — this is a "there is no
    /// pivot here" test, not a quality test, which the [`STABILITY`] check
    /// next to it already is.
    pub(crate) const REBUILD_MIN_PIVOT: f64 = 1e-12;

    /// Caps on [`factorize_reusing`]'s own exponential backoff after a
    /// rejected reuse: the streak's shift is capped first (so the shift itself
    /// can never overflow), then the resulting skip count.
    pub(crate) const REUSE_BACKOFF_SHIFT_CAP: u32 = 5;

    pub(crate) const REUSE_MAX_BACKOFF: u32 = 16;

    /// A column/row whose off-diagonal fill exceeds this fraction of `m` is
    /// stored densely (see [`HybridVec`]). Unlike [`DENSE_COL_FRACTION`] (tuned
    /// against real Netlib data, all of it sparse), this threshold has no
    /// dense-problem benchmark to tune against yet in this crate's own test
    /// set — `0.4` is a first-pass value, not a measured one; re-tune once a
    /// genuinely dense-coefficient LP is available to benchmark against.
    pub(crate) const DENSE_ETA_FRACTION: f64 = 0.4;

    /// A caller-provided FTRAN right-hand side whose own nonzero count exceeds
    /// this fraction of `m` is dense enough that the Gilbert-Peierls sparse
    /// path's DFS/epoch bookkeeping (see `LuFactors::l_solve_sparse_into`'s own
    /// docs) no longer pays for itself — its reach set is bounded below by the
    /// rhs's own nonzero count, so a dense rhs alone already guarantees a large
    /// reach regardless of how sparse `L` itself is. Exposed as
    /// [`FtLu::should_use_dense_solve`] rather than a flag fixed at
    /// construction time: an earlier version of this gate measured density
    /// once per refactorization from the *basis*'s own `L`/`U` fill and cached
    /// it — which reads as permanently sparse for the entire solve whenever
    /// the crash-start basis (the slack identity, always maximally sparse)
    /// never gets refactorized a second time, silently never firing even on a
    /// genuinely dense-coefficient LP whose real (post-pivoting) basis is
    /// dense throughout. Checking the actual rhs at each call site instead has
    /// no such staleness problem and costs nothing extra (the caller already
    /// has the sparse rhs's length on hand). Like `DENSE_ETA_FRACTION`, `0.4`
    /// is a first-pass threshold, not one tuned against a real dense-problem
    /// benchmark yet.
    pub(crate) const DENSE_RHS_FRACTION: f64 = 0.4;

    /// Weight given to the newest observation when folding it into an
    /// [`FtranDensity`] running average. This is HiGHS's own
    /// `kRunningAverageMultiplier` (`HEkk::updateOperationResultDensity`,
    /// used there for exactly the same purpose — see that class's
    /// `col_aq_density`/`row_ep_density` fields), kept at the same value for
    /// the same reason: small enough that one atypical iteration cannot flip
    /// the dense/sparse dispatch on its own, large enough that a genuine
    /// phase change (a basis that has filled in over the last dozen pivots)
    /// is picked up within ~20 iterations rather than being averaged away
    /// over the whole solve.
    pub(crate) const DENSITY_AVERAGE_MULTIPLIER: f64 = 0.05;

    /// An FTRAN call site whose recent *results* have averaged denser than
    /// this fraction of `m` takes the dense solve regardless of how sparse
    /// the right-hand side it is handed happens to be — see [`FtranDensity`]'s
    /// own docs for why the input's own nonzero count
    /// ([`DENSE_RHS_FRACTION`]) is not a sufficient predictor on its own.
    /// Overridable at run time via `ENOMOTO_EXPECTED_DENSITY_GATE` (see
    /// [`expected_dense_gate`]) so this one number can be re-tuned against the
    /// Netlib set without a rebuild.
    ///
    /// `0.35` is measured, not guessed (`analysis/ftran_density_gate_20260922_062832.md`
    /// §4.2, an A/B over the full Netlib set run *inside one process* with the
    /// setting flipped between solves, since this box's per-problem run-to-run
    /// spread otherwise reaches 4x): against the gate disabled, `0.35` is -3.9%
    /// over the 14 mid-heavy instances and -1% over all 93, while `0.2` is
    /// *worse* than no gate at all (+0.8%). The reason `0.2` loses is specific
    /// and worth keeping: it drags the BFRT combined-flip channel onto the dense
    /// path too (its results average 0.24-0.49 dense, against the entering
    /// column's 0.65-0.99), and on `greenbeb` that turns a -22% win into -1%.
    /// A threshold between the two channels' own measured densities is what the
    /// gate wants, not the lowest one that still fires.
    pub(crate) const EXPECTED_DENSE_FRACTION: f64 = 0.35;

    /// Density ceiling for BTRAN's row-major scatter form
    /// ([`LuFactors::l_transpose_solve_scatter_into`]): the `L^{-T}` stage
    /// takes it only when under this fraction of the incoming `w` is nonzero,
    /// and falls back to the column-major gather form
    /// ([`LuFactors::l_transpose_solve_gather_into`]) otherwise.
    ///
    /// **A gate is needed here, not just a faster kernel.** The two forms
    /// touch exactly the same `L` entries; what differs is the access shape.
    /// Gather reads `w[row_step]` at random and accumulates into one place
    /// (`w[s]`, which the compiler keeps in a register across the whole inner
    /// loop); scatter reads one place (`w[s]`) and does a random
    /// read-modify-write per entry. On a sparse `w` the scatter's whole-step
    /// skip wins outright — most steps do no work at all — but on a dense `w`
    /// nothing is skipped and the scatter is left paying random *stores*
    /// where the gather paid random *loads*, which is strictly worse. Measured
    /// exactly that way on the first ungated A/B of this change: `dfl001`
    /// (whose BTRAN `w` is dense by the time `U^{-T}` and the `R` etas are
    /// done with it) +6.5%, against wins on the sparse-`w` instances. HiGHS
    /// gates all four of its own solve directions for the same reason
    /// (`HFactor::btranL`'s own `sparse_solve` test, `kHyperBtranL`).
    ///
    /// The test is an exact nonzero count of `w`, not a running-average
    /// prediction: unlike an FTRAN's input (whose density is only knowable
    /// from history — see [`FtranDensity`]'s own docs), `w` is right there in
    /// a buffer that every path over it already scans at least once more
    /// (the permutation into `y`), so one early-exiting `O(m)` sequential
    /// pass answers the question exactly, for a fraction of the `nnz(L)`
    /// random accesses the stage itself is about to do either way.
    pub(crate) const BTRAN_L_SCATTER_FRACTION: f64 = 0.10;

    /// Per-row-of-`U`-and-`L` coefficient for [`FtLu::build_tick`]'s `m`-only
    /// term — HiGHS's own `buildSynthticTick` (`HFactor.cpp`) uses `80` for the
    /// analogous term (`num_row * 80`); kept unchanged here rather than
    /// re-derived, since this crate's `refactorize` pays the same *kind* of
    /// fixed per-row bookkeeping (permutation arrays, `u_seq`/`row_owners`
    /// construction in [`FtLu::new`]) HiGHS's own `buildFinish` does, just at a
    /// different (higher, per `docs/lu_comparison_enomoto_vs_highs.md` §3.1 and
    /// this trigger's own analysis §4) constant of proportionality that the
    /// *other* coefficient ([`TICK_BUILD_LU_COEF`]) already carries — see
    /// [`SYNTH_CLOCK_FACTOR`]'s own docs for why the *ratio* between the two
    /// build-tick terms and the *solve*-side tick units is what calibration
    /// actually tunes, not this constant in isolation.
    pub(crate) const TICK_BUILD_M_COEF: u64 = 80;

    /// Per-nonzero-of-`(L+U)` coefficient for [`FtLu::build_tick`] — HiGHS's own
    /// `buildSynthticTick` uses `60` for `(l_nnz + u_off) * 60`. Kept at HiGHS's
    /// own value for the same reason as [`TICK_BUILD_M_COEF`]: this crate's
    /// Markowitz `factorize` (§4/§5 of this trigger's own analysis, measured
    /// when the kernel still used `BTreeMap`/`BTreeSet` storage rather than
    /// today's [`KernelMatrix`]) is 3-25x more expensive *per nonzero* than
    /// HiGHS's `HFactor::buildKernel` — the flattening narrowed that gap but
    /// did not close it, and it is the gap's *existence*, not its exact size,
    /// that makes a
    /// *higher* [`SYNTH_CLOCK_FACTOR`] (not a higher `TICK_BUILD_*_COEF`) the
    /// right lever: raising these two coefficients would inflate `build_tick`
    /// but leave the *solve*-side tick (driven by [`TICK_SOLVE_NNZ_COEF`]) at
    /// the same scale, which double-counts the same "our factorization is
    /// slower" fact the factor calibration already absorbs once.
    pub(crate) const TICK_BUILD_LU_COEF: u64 = 60;

    /// Per-multiply-add coefficient of the elimination flop term in
    /// [`FtLu::build_tick`] (S16, `ENOMOTO_T_TICK_BUILD_FLOP_COEF`). `0` (the
    /// default) leaves `build_tick` exactly the HiGHS-shaped `m`/`nnz(L+U)` sum.
    pub(crate) const TICK_BUILD_FLOP_COEF: u64 = 0;

    /// Per-nonzero coefficient applied to every solve-stage tick increment
    /// (`R`-eta nonzeros touched, `U`/`U^T`-eta nonzeros touched, `L`-stage
    /// reach-set size) — kept at `1` (i.e. `tick` is a plain nonzero count,
    /// unscaled) so [`SYNTH_CLOCK_FACTOR`] alone carries the crate-specific
    /// per-nonzero cost ratio between this crate's own solves and HiGHS's; splitting that ratio across two constants
    /// (this one and the factor) would make calibration harder to reason about
    /// with no accuracy benefit, since both only ever appear multiplied
    /// together in the trigger's own comparison.
    pub(crate) const TICK_SOLVE_NNZ_COEF: u64 = 1;
}

/// 前処理 (src/presolve.rs, src/presolve/*.rs)
pub(crate) mod presolve {
    // ---- 共通 ----

    /// 前処理全般で「0 とみなす」係数・差の絶対許容誤差 (ピボットが 0 か、係数が
    /// 実質 0 か、行が空か等の判定)。
    /// 【元: aggregator, colsingleton, dominatedcol, doubleton, dualfix, dualpropagate,
    /// foldfixed, freevar, ineqsingleton, parallelcols, parallelrows, rowdominance,
    /// rowsingleton, sparsify, stuffing の各 `TOL` を統合】
    pub(crate) const TOL: f64 = 1e-9;

    /// 代入消去のピボット判定: 消去に使う係数が `|coeff| >= この値 * max|row|` を
    /// 満たさなければその行では消去しない (小さいピボットで割ると復元時の誤差が増幅される)。
    /// colsingleton / freevar / aggregator で共通 (`ENOMOTO_T_*SUBSTITUTION_PIVOT_RATIO`)。
    pub(crate) const SUBSTITUTION_PIVOT_RATIO: f64 = 1e-2;

    /// 境界保存行が残りの項の箱制約から既に含意されているかを判定する相対許容誤差
    /// (colsingleton / doubleton)。
    pub(crate) const IMPLIED_TOL: f64 = 1e-9;

    // ---- aggregator ----

    /// aggregator: 1 列の消去で増えてよい非零要素数の上限 (HiGHS の
    /// `presolve_substitution_maxfillin` の既定値)。超える列は見送る。
    pub(crate) const MAX_FILLIN: usize = 10;

    /// aggregator: fill-in 超過による却下がこの回数連続したら、その呼び出しの残り候補を
    /// 打ち切る (HiGHS の `nfail == 3`)。
    pub(crate) const MAX_CONSECUTIVE_FILLIN_FAILURES: usize = 3;

    // ---- propagate ----

    /// 上下限伝播の絶対許容誤差 (上下限の更新幅・矛盾判定・行の冗長判定に使う)。
    pub(crate) const PROPAGATE_EPS: f64 = 1e-9;

    /// 不等式伝播 (`propagate_split`) の相対改善閾値 (`ENOMOTO_T_PROP_RELTOL`)。
    /// 有限の境界は改善幅が `reltol * (1 + |bound|)` を超えるときだけ更新する。
    /// 0 で無効 (絶対閾値 `PROPAGATE_EPS` のみ)。
    pub(crate) const PROP_RELTOL: f64 = 0.0;

    /// 等式伝播 (`propagate_equalities`) の相対改善閾値 (`ENOMOTO_T_EQPROP_RELTOL`)。
    /// 有限の境界は変化量が `reltol * (1 + |old|)` を超えるときだけ更新する。0 で無効。
    pub(crate) const EQPROP_RELTOL: f64 = 0.0;

    // ---- redundancy (等式行の一次従属検出) ----

    /// 重複除去後の等式行の非零密度 `nnz / (p * n)` がこれを超えたら密 QR
    /// (`drop_linearly_dependent`)、以下なら疎消去 (`drop_linearly_dependent_sparse`) を使う。
    pub(crate) const DENSE_DENSITY_THRESHOLD: f64 = 0.03;

    /// 密 QR 経路 (`drop_linearly_dependent`) の従属判定の相対許容誤差。`|R[k,k]|` が
    /// その行自身の拡大ノルム `||[係数; 右辺]||_2` のこの倍数以下なら一次従属として落とす
    /// (`ENOMOTO_T_REDEQ_QR_RANK_TOL`)。
    pub(crate) const REDEQ_QR_RANK_TOL: f64 = 1e-9;

    /// 疎消去でピボット候補が満たすべき安定性の下限。行列全体の現在の最大絶対値に
    /// 対する比で判定する (列内最大でなく全体最大に対する比なので階数判定として正しい)。
    pub(crate) const PIVOT_STABILITY: f64 = 0.1;

    /// 疎消去で行を一次従属 (冗長) とみなす閾値。ピボット値が
    /// `DEP_TOL * (その行の元のノルム)` 以下なら従属と判定する。
    pub(crate) const DEP_TOL: f64 = 1e-9;

    /// ブロック分解後の非自明ブロック (サイズ > 1) の総行数がこれ以上なら、
    /// ブロックごとの消去を rayon で並列実行する。
    pub(crate) const PARALLEL_DECOMPOSE_ROW_THRESHOLD: usize = 64;

    /// 等式行数がこれ未満なら Dulmage-Mendelsohn ブロック分解を省き、
    /// 疎消去を直接呼ぶ (小さい問題では分解のオーバーヘッドが得にならない)。
    pub(crate) const MIN_ROWS_FOR_BLOCK_DECOMPOSE: usize = 300;

    // ---- scaling ----

    /// scaling: A と G の合計行数がこれを超えたら列ノルム集計を rayon で並列化する
    /// (`simplex.rs` の `RAYON_SIZE_THRESHOLD` と同値の独立コピー)。
    pub(crate) const RAYON_SIZE_THRESHOLD: usize = 100_000;

    /// scaling: 行・列のノルムがこれ以下なら 0 とみなしてスケールを更新しない
    /// (`ENOMOTO_T_SCALING_ZERO_TOL`)。
    pub(crate) const SCALING_ZERO_TOL: f64 = 1e-12;

    /// scaling: G の単一要素行 (箱制約行) を専用リストで処理する高速経路を使うか
    /// (1 = 使う。結果は通常経路とビット一致。`ENOMOTO_T_SCALE_UNIT_FAST`)。
    pub(crate) const SCALE_UNIT_FAST: usize = 1;

    /// scaling: 箱制約行を Ruiz 反復から除外し閉形式のスケールを与える実験的経路を使うか
    /// (0 = 使わない。スケール係数が変わる。`ENOMOTO_T_SCALE_NOBOUNDS`)。
    pub(crate) const SCALE_NOBOUNDS: usize = 0;

    // ---- smallcoeff ----

    /// smallcoeff: 無視できる主実行不能量の基準 eps (単体法の `PRIMAL_FEAS_TOL` 相当の独立コピー)。
    pub(crate) const SMALLCOEFF_EPS: f64 = 1e-7;

    /// smallcoeff: 1 行あたりに許す係数除去による最悪活動度変化の合計の上限
    /// (`SMALLCOEFF_EPS` に対する比。Achterberg et al. の `1e-1 * eps`)。
    pub(crate) const CUMULATIVE_FRACTION: f64 = 0.1;

    /// smallcoeff: 絶対値がこれ以下の係数は予算と無関係に除去する (Achterberg et al. の `1e-10`)。
    pub(crate) const NOISE_THRESHOLD: f64 = 1e-10;

    // ---- パイプライン (src/presolve.rs の run_extended) ----

    /// 等式の冗長行削除の方式 (`ENOMOTO_REDEQ_MODE`)。
    /// 0 = ラウンド前に完全版 (重複 + 階数判定)、1 = 重複削除のみ、
    /// 2 = ラウンド前に重複削除、ラウンド後の縮小問題で階数判定。
    pub(crate) const REDEQ_MODE: usize = 1;

    /// G を上下限と多変数行に分離したまま保持するか (1 = 保持。0 だと毎回 CSR を構築。
    /// `ENOMOTO_T_PRESOLVE_SPLIT_G`)。
    pub(crate) const PRESOLVE_SPLIT_G: usize = 1;

    /// 等式行による上下限伝播を行う外側ラウンド数 (最初のこの回数だけ。`ENOMOTO_T_EQPROP_ROUNDS`)。
    pub(crate) const EQPROP_ROUNDS: usize = 2;

    /// 等式行伝播が 1 回でも何も見つけなければ残りのラウンドで省略するか
    /// (0 = 省略しない。`ENOMOTO_T_EQPROP_SKIP_IDLE`)。
    pub(crate) const EQPROP_SKIP_IDLE: usize = 0;

    /// dualpropagate がこの回数連続で何も見つけなければ以降のラウンドで停止する
    /// (`ENOMOTO_T_DUALPROPAGATE_STRIKES`)。
    pub(crate) const DUALPROPAGATE_STRIKES: usize = 1;

    /// doubleton がこの回数連続で何も消去しなければ以降のラウンドで停止する
    /// (`ENOMOTO_T_DOUBLETON_STRIKES`)。
    pub(crate) const DOUBLETON_STRIKES: usize = 1;

    /// parallelcols がこの回数連続で何も併合しなければ以降のラウンドで停止する
    /// (`ENOMOTO_T_PARALLELCOLS_STRIKES`)。
    pub(crate) const PARALLELCOLS_STRIKES: usize = 2;

    /// 構造 (行数・固定列数・ログ長) が前ラウンドと同じなら上下限の変化を無視して
    /// ラウンドを打ち切るか (0 = しない。`ENOMOTO_T_ROUND_STRUCT_STOP`)。
    pub(crate) const ROUND_STRUCT_STOP: usize = 0;

    /// 外側ラウンドの不動点判定で、上下限の変化を進展とみなす相対閾値。
    /// `|u - v| <= この値 * (1 + max(|u|, |v|))` の変化は無視する (`ENOMOTO_T_FIXPOINT_RELTOL`)。
    pub(crate) const FIXPOINT_RELTOL: f64 = 1e-3;

    // ==== 以下: presolve/ の小規模モジュール (dualfix, dualpropagate, foldfixed, ineqsingleton,
    // parallelcols, parallelrows, rowdominance, rowsingleton, smallcoeff, sparsify, stuffing,
    // dominatedcol) のコード中から新たに切り出した定数 ====
}

/// 内点法 IP-PMM (src/interior_point.rs と interior_point/kkt.rs)。
/// 既定のエンジンではなく、`Model.solve(root_solver="interior")` のときだけ使われる。
pub(crate) mod interior_point {
    /// fraction-to-boundary 則の係数 τ。スラック `s` と双対 `z` が 0 に達しないよう、
    /// 境界までの最大ステップの τ 倍までしか進まない。
    pub(crate) const TAU: f64 = 0.995;

    /// 主変数側の近接正則化パラメータ ρ の下限。
    pub(crate) const RHO_MIN: f64 = 1e-10;

    /// 双対側の近接正則化パラメータ δ の下限。
    pub(crate) const DELTA_MIN: f64 = 1e-10;

    /// ρ の初期値。
    pub(crate) const RHO0: f64 = 1e-1;

    /// δ の初期値。
    pub(crate) const DELTA0: f64 = 1e-1;

    /// 停止判定 (主・双対残差と双対ギャップ) の絶対許容誤差。
    pub(crate) const EPS_ABS: f64 = 1e-8;

    /// 停止判定の相対許容誤差 (各量のノルムに掛ける)。
    pub(crate) const EPS_REL: f64 = 1e-8;

    /// Newton 反復の最大回数。
    pub(crate) const MAX_ITERS: usize = 100;

    /// 正則化が下限に張り付いたまま残差が改善しない反復がこの回数続いたら停滞とみなし、
    /// 残差の大小で実行不能/非有界を推定して打ち切る。
    pub(crate) const STALL_ITERS: usize = 8;

    /// 前処理 (`presolve::run_extended`) に渡す Ruiz 平衡化の反復回数。
    pub(crate) const RUIZ_ITERS: usize = 10;

    /// 冗長等式行の除去後に、不等式行の制約伝播 (境界強化 + 冗長/実行不能行の検出) を
    /// 何回繰り返すか。
    pub(crate) const PROPAGATION_PASSES: usize = 2;

    /// `presolve::run_extended` の外側ラウンド
    /// (propagate → dualfix → 行シングルトン → ダブルトン → 列シングルトン) の上限回数。
    /// 収束すればそれより早く止まる。
    pub(crate) const PRESOLVE_ROUNDS: usize = 20;

    /// 外側ラウンド 1 回の中で、行シングルトン ⇔ 列シングルトンの組を繰り返す上限回数。
    pub(crate) const ROWSINGLETON_COLSINGLETON_INNER_ROUNDS: usize = 1;

    /// Farkas 証明の判定で、候補ベクトルの無限大ノルムがこれ未満なら
    /// 「ほぼ零ベクトル」とみなして証明なしと判定する。
    pub(crate) const CERT_SCALE_MIN: f64 = 1e-8;

    /// Farkas 証明の判定許容誤差 (正規化後の残差がこれ未満、かつ目的の改善がこれ超なら成立)。
    pub(crate) const CERT_TOL: f64 = 1e-5;

    /// 不等式が 1 本もない (`m == 0`) 場合、初期化系の双対残差の無限大ノルムが
    /// これを超えたら非有界と判定する。
    pub(crate) const NO_INEQ_DUAL_RES_TOL: f64 = 1e-4;

    /// 初期点のシフト量 `0.5 * (-min) * この値` に使う倍率 (PIQP の初期化式)。
    pub(crate) const INIT_SHIFT_MULTIPLIER: f64 = 3.0;

    /// 初期点シフトの割り算で分母 (総和) にかける下限 (ゼロ除算よけ)。
    pub(crate) const INIT_DIV_GUARD: f64 = 1e-12;

    /// 初期スラック `s` と初期双対 `z` の各成分の下限 (厳密に正の内点から始めるため)。
    pub(crate) const INIT_POSITIVE_FLOOR: f64 = 1e-6;

    /// 「ρ, δ が下限に達した」とみなす余裕倍率 (下限 × この値 以下なら到達)。
    pub(crate) const REG_FLOOR_SLACK: f64 = 1.001;

    /// 停滞判定: 今回の残差が前回の残差 × この値 以上なら「改善なし」とみなす。
    pub(crate) const STALL_PROGRESS_RATIO: f64 = 0.999999;

    /// 相補性 μ や双対ギャップで割るときの分母の下限 (ゼロ除算よけ)。
    pub(crate) const GAP_DIV_GUARD: f64 = 1e-16;

    /// 残差が前回の この倍率 以下まで減ったら近接中心 (λ, ν または ξ) を更新し、
    /// 正則化パラメータを大きく減らす。
    pub(crate) const RES_DECREASE_RATIO: f64 = 0.95;

    /// 残差が十分減らなかったときの正則化パラメータの減らし方
    /// (`(1 - r / この値)` 倍、`r` は相補性ギャップの相対減少率)。
    pub(crate) const SLOW_DECREASE_DIVISOR: f64 = 3.0;

    /// KKT 系の数値 LDLᵀ 分解と三角求解に使う faer の並列度。
    /// `Rayon(0)` は rayon のスレッド数をそのまま使う指定。
    /// スクラッチ量の見積もり (`_req`) と実際の呼び出しで同じ値を使う必要がある。
    pub(crate) const KKT_PARALLELISM: faer::Parallelism = faer::Parallelism::Rayon(0);
}

/// 分枝限定法 (src/mip.rs)
pub(crate) mod mip {
    /// 整数変数の緩和解が整数からこの距離以内なら「整数値」とみなす。
    pub(crate) const INT_TOL: f64 = 1e-6;

    /// 暫定解を置き換えるのに必要な目的関数値の最小改善量 (浮動小数点の雑音で
    /// 暫定解が入れ替わり続けるのを防ぐ)。
    pub(crate) const OBJ_EPS: f64 = 1e-7;

    /// 探索ノード数の上限。超えたらその時点の最良暫定解を `node_limit_hit: true` 付きで返す。
    pub(crate) const MAX_NODES: usize = 20_000;

    /// 分枝で締めた境界が `下限 > 上限 + この値` になったら、そのノードを実行不能として捨てる。
    pub(crate) const BOUND_INFEAS_TOL: f64 = 1e-9;
}

/// メモリアロケータ (src/lib.rs)
pub(crate) mod alloc {
    /// このバイト数未満のメモリブロックは mimalloc に、以上はシステムアロケータに割り当てる。
    pub(crate) const MIMALLOC_SIZE_LIMIT: usize = 4 * 1024;
}
