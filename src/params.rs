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
    /// 戻すときのスナップ幅にも使う (Gill et al. 1989 §4.2-4.3)。
    pub(crate) const EXPAND_DELTA_F: f64 = 1e-6;

    /// EXPAND の 1 拡大系列の反復数 (この反復数ごとにリセットする, Gill et al. 1989 §4.2)。
    pub(crate) const EXPAND_K: usize = 50;

    /// 拡大系列の開始時の作業許容誤差 (Gill et al. 1989 §4.2: `delta_0 = 0.5 delta_f`)。
    pub(crate) const EXPAND_DELTA_0: f64 = 0.5 * EXPAND_DELTA_F;

    /// `EXPAND_K` 反復で近づく作業許容誤差の上限 (Gill et al. 1989 §4.2: `delta_K = 0.99 delta_f`)。
    pub(crate) const EXPAND_DELTA_K: f64 = 0.99 * EXPAND_DELTA_F;

    /// 作業許容誤差の 1 反復あたりの増分 (Gill et al. 1989 §4.2: `tau = (delta_K - delta_0) / K`)。
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

    /// 費用摂動: 費用が全部 0 の問題で「最大費用」の代わりに使う値。0 のままだと摂動が消えて
    /// 双対退化で反復が大きく増える(klein2: 1740 → 214 反復)。
    pub(crate) const COST_PERTURB_ZERO_COST_SCALE: f64 = 1.0;
}

/// 傾き・切片二段解法 (src/simplex/slope_intercept_dual.rs)
pub(crate) mod slope_intercept_dual {
    /// `x_B(M)` の増分維持値のドリフト検査(と eta フィル検査)を行う主ループの反復間隔。
    /// `fill_count` にかかわらず毎回検査し、古典法のように `RESIDUAL_CHECK_MULTIPLIER` で
    /// さらに間引くことはしない(比較の多くが傾き項で `LEX_REL_TOL = 1e-9` という厳しさで決まるため)。
    /// 値は `simplex::FT_CHECK_INTERVAL`(5)と同じ。「再分解後の最初の検査」の判定
    /// (`updates <= XB_CHECK_INTERVAL`)にも使う。実際の検査の間隔は [`XB_CHECK_CADENCE`]。
    pub(crate) const XB_CHECK_INTERVAL: usize = super::simplex::FT_CHECK_INTERVAL;

    /// 主ループで `x_B(M)` のドリフト検査(と eta フィル検査)を行う反復間隔。2026-09 に
    /// [`XB_CHECK_INTERVAL`](5)から 20 に再調整した(`PIVOT_SEARCH_LIMIT = 64`・主 handoff の
    /// 増分化と Devex 化と合わせて Netlib 93 問 -8.3%、全問一致)。`XB_CHECK_INTERVAL` ごと 20 に
    /// すると再分解後最初の検査の判定まで変わり、dfl001・pilot87 が遅くなって効果の大半が消えた。
    /// `ENOMOTO_T_XB_CHECK_INTERVAL` で上書き可。
    pub(crate) const XB_CHECK_CADENCE: usize = 20;

    /// 策9(stormG2 報告 §4): 行数 `m` がこれ以上の問題では、ドリフト検査の間隔を
    /// [`XB_CHECK_CADENCE_LARGE`] にする(検査 1 回が `O(m + nnz(A_B))` で、m が数十万行では
    /// 20 反復ごとだと反復本体より重くなるため)。Netlib(`m` ≤ 約 6K)には掛からない。
    /// `ENOMOTO_T_XB_CHECK_LARGE_M` で上書き可(0 = 無効)。
    pub(crate) const XB_CHECK_CADENCE_LARGE_M: usize = 10_000;

    /// 策9: 大きな問題でのドリフト検査の反復間隔の下限(`ENOMOTO_T_XB_CHECK_CADENCE_LARGE`)。
    /// 間隔は `max(この値, m / XB_CHECK_CADENCE_LARGE_DIV)`。
    pub(crate) const XB_CHECK_CADENCE_LARGE: usize = 100;

    /// 策9: 大きな問題でのドリフト検査の間隔を `m` に比例させる除数(`ENOMOTO_T_XB_CHECK_CADENCE_LARGE_DIV`)。
    /// 検査 1 回の手間は `O(m + nnz(A_B))` なので、反復あたりの償却コストを `m` によらず一定にする
    /// (m=76K で約 300、m=378K で約 1,480 反復ごと)。storm k=200 で間隔 100 → 300 は総時間 -11%。
    pub(crate) const XB_CHECK_CADENCE_LARGE_DIV: usize = 256;

    /// 策12(部分 `tau`): 行数 `m` がこれ以上の問題では、融合 FTRAN の DSE `tau = B^-1 rho` を
    /// 入る列の結果の非ゼロ行(DSE 重み更新が読む行)でだけ求める(`ENOMOTO_T_PARTIAL_TAU_MIN_M`、0 = 無効)。
    /// 求めた値はビット一致だが CLOCK tick の数え方が変わる(再分解の時期が変わりうる)ので、
    /// Netlib(`m` ≤ 約 6K)には掛からないようにしている。
    pub(crate) const PARTIAL_TAU_MIN_M: usize = 10_000;

    /// `x_B(M)` のドリフト検査の許容誤差(残差 `‖A_B x_B - rhs‖` の絶対値、基底・傾きの両チャネル)。
    /// 比較が `1e-9` の相対許容誤差で決まるので、古典法の `FT_RESIDUAL_TOL`(1e-4)より十分厳しくする。
    /// 1 回の求解内で、ドリフト起因の再分解が [`XB_DRIFT_ESCALATION_STEP`] 回起きるごとに
    /// [`XB_DRIFT_ESCALATION_FACTOR`] 倍に緩める(上限 [`XB_DRIFT_TOL_MAX`])。再分解が少ない求解は
    /// この値のまま。`ENOMOTO_XB_DRIFT_TOL` で上書き可。
    pub(crate) const XB_DRIFT_TOL: f64 = 1e-8;

    /// 同じ求解内でドリフト起因の再分解がこの回数起きるごとに、[`XB_DRIFT_TOL`] の実効値を
    /// [`XB_DRIFT_ESCALATION_FACTOR`] 倍にする(上限 [`XB_DRIFT_TOL_MAX`])。ただし再分解後最初の
    /// 検査で既に超えたとき(分解し直しても下がらない)は、回数を待たず次の段へ進める。
    pub(crate) const XB_DRIFT_ESCALATION_STEP: usize = 10;

    /// `x_B(M)` ドリフト検査の相対許容誤差(0 = 無効)。残差が絶対許容誤差を超えても、
    /// `XB_DRIFT_REL_TOL × ‖|A_B||x_B| + |rhs|‖`(`A_B x_B` の丸め誤差の尺度)以下なら再分解しない。
    /// 絶対許容誤差 1e-8 は `‖x_B‖` が大きい問題では相対 1e-15 程度になり、丸めノイズだけで
    /// 発火して再分解を繰り返す(klein3: ‖x_B‖ が 2e5 → 9e8 に育ち、再分解 15 → 5 回)。
    /// 1e-14 で Netlib 93 問 -3.1%(pilot87 -6%、pilot -11%)、全問一致。1e-15 は差なし。
    /// `ENOMOTO_XB_DRIFT_REL_TOL` で上書き可(0 で無効)。
    pub(crate) const XB_DRIFT_REL_TOL: f64 = 1e-14;

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

    /// 策10(stormG2 報告 §4): 行数 `m` がこれ以上の問題では、合成クロックの係数を
    /// `SYNTH_CLOCK_FACTOR * sqrt(m / SYNTH_CLOCK_LARGE_REF_M)` に広げる
    /// (`ENOMOTO_T_SYNTH_CLOCK_LARGE_M`、0 = 無効)。求解の `O(m)` パスを消した後も tick は段ごとに
    /// 一律 `m` を数えるので再分解間隔は `m` によらずほぼ一定だが、再分解 1 回の手間は `m` に比例し、
    /// 更新 1 回ごとに増える `R` eta の手間は `m` によらない。両者の釣り合う間隔は `sqrt(m)` に比例する。
    /// Netlib(`m` ≤ 約 6K)には掛からない。
    pub(crate) const SYNTH_CLOCK_LARGE_M: usize = 10_000;

    /// 策10: 係数を広げるときの基準行数(`ENOMOTO_T_SYNTH_CLOCK_LARGE_REF_M`)。
    /// 当初は 5,000(storm 縮小版で m=19K → 係数約 31、m=76K → 約 62 が最良付近)。疎な `R` 段(追加策 R)で
    /// 更新 1 回ごとの増分が減った後は 2,000 が良い(本体 m=378K: 5,000 / 2,000 / 1,000 で 92 / 84 / 87 s、
    /// k=200: 5.95 / 5.41 / 5.27 s、k=50: 0.79 / 0.74 / 0.76 s)。
    pub(crate) const SYNTH_CLOCK_LARGE_REF_M: usize = 2_000;

    /// 増分維持している被約費用 `d` のドリフト検査の相対許容誤差: `‖d - fresh_d‖`(固定列を除く)が
    /// `D_DRIFT_TOL * max(‖fresh_d‖, 1)` を超えたら再分解する。
    pub(crate) const D_DRIFT_TOL: f64 = 1.0;

    /// PRICE によるピボット要素 `alpha_q` と FTRAN による `alpha_full[r]` の相対差がこれを超えたら
    /// 「桁違いの不一致」(`pivot_grossly_inconsistent`)としてピボットを破棄する。
    /// `pivot_values_agree` の厳しい許容誤差(1e-7)よりずっと緩く、`update_count` によらず常に検査する。
    pub(crate) const D_GROSS_MISMATCH_REL_TOL: f64 = 0.5;

    /// 直前のピボットが updateVerify で破棄された行(`stuck_row`)を再試行するときの比率テストの
    /// ピボット下限 `|alpha_j|`。破棄直後の再分解で `update_count == 0` になり厳しい照合が
    /// 効かないため、ここで 1e-9 級の極小ピボットを選ぶと基底が特異に近づき、verify 失敗の連鎖の末に
    /// 再分解が特異で失敗する(cplex2 が `NotSolved`)。FT 更新が拒否する大きさ([`super::simplex::FT_MIN_PIVOT`])に
    /// 揃える。この下限を満たす候補が無いときは、他に実行不能行があればこの行を一時的に外し
    /// (常に行う。cplex2 はこれで最初の求解で解ける)、無ければ通常の [`super::simplex::TOL`] の
    /// 候補をそのまま使う(候補を空にすると `Eligible = ∅` の実行不能判定を誤って下すため)。
    /// 下限を満たす候補があるとき小さい候補を除く方は、特異基底で破綻した求解を解き直す
    /// 安全モード(`safe_pivot`)でだけ行う(経路を変える範囲が広いため)。
    pub(crate) const STUCK_ROW_MIN_PIVOT: f64 = 1e-7;

    /// `x_B(M)` の `M` 係数は厳密には 0 か 1 のオーダーなので、絶対値がこれ未満の係数は
    /// LU/更新の雑音とみなして 0 に丸める(`snap_slope`)。`ENOMOTO_T_X_B_SLOPE_NOISE` で上書き可。
    pub(crate) const X_B_SLOPE_NOISE: f64 = 1e-7;

    /// `M` 係数の絶対許容誤差: 段階 A → B の移行で、基底の `x^1_j` が傾き境界 `l^1_j`/`u^1_j` 上に
    /// あるとみなす(その境界を `l^B`/`u^B` に残す)範囲。`Affine1::gt_zero` の傾き閾値と同じ値。
    pub(crate) const SLOPE_TOL: f64 = 1e-9;

    /// 最適値の傾き `z^1`(`z(M) = z^0 + z^1 M`)が `z^1 < 0`(実行不能または非有界、
    /// 論文の系 7.3 (i))とみなされる負の閾値(`-Z_SLOPE_TOL` 未満)。段階 A の早期終了と
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

    /// 策7(stormG2 報告 §4): 行数 `m` がこれ以上の問題では、`CHUZR_SHORTLIST_K == 0`(既定)でも
    /// S11 の短縮リストを長さ `CHUZR_SHORTLIST_AUTO_K` で有効にする(`ENOMOTO_T_CHUZR_SHORTLIST_AUTO_MIN_M`、
    /// 0 = 無効)。Netlib(`m` ≤ 約 6K)には掛からないので経路は変わらない。
    pub(crate) const CHUZR_SHORTLIST_AUTO_MIN_M: usize = 10_000;

    /// 策7: 自動で有効にした短縮リストの長さ `K` の下限(`ENOMOTO_T_CHUZR_SHORTLIST_AUTO_K`)。
    /// 全走査のたびに `K = clamp(プール行数 / CHUZR_SHORTLIST_AUTO_DIV, この値, CHUZR_SHORTLIST_AUTO_K_MAX)`。
    pub(crate) const CHUZR_SHORTLIST_AUTO_K: usize = 64;

    /// 策7: 自動モードの `K` をプール行数から決めるときの除数(`ENOMOTO_T_CHUZR_SHORTLIST_AUTO_DIV`)。
    pub(crate) const CHUZR_SHORTLIST_AUTO_DIV: usize = 64;

    /// 策7: 自動モードの `K` の上限(`ENOMOTO_T_CHUZR_SHORTLIST_AUTO_K_MAX`)。リストの走査は毎反復
    /// `O(K)` なので大きくしすぎない。
    pub(crate) const CHUZR_SHORTLIST_AUTO_K_MAX: usize = 512;

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
///
/// 各定数の測定経緯は `docs/improvement_history.md` を参照。
pub(crate) mod lu {
    // ---- ピボット選択 (Markowitz) ----

    /// 閾値ピボットの安定性下限: ピボット候補は、その列の活性部分の最大絶対値の
    /// この割合以上でなければならない。求解ごとのピボット閾値
    /// (`sparse_lu::pivot_threshold`) の初期値
    /// (引き上げは既定 off なので実質この値で固定)。
    pub(crate) const STABILITY: f64 = 0.25;

    /// ピボット閾値を引き上げる際の上限 (HiGHS の `kMaxPivotThreshold`)。
    pub(crate) const PIVOT_THRESHOLD_MAX: f64 = 0.5;

    /// 環境変数 `ENOMOTO_PIVOT_THRESHOLD` で指定されたピボット閾値の下限
    /// (HiGHS の `kMinPivotThreshold`)。上書き値のクランプにだけ使う。
    pub(crate) const PIVOT_THRESHOLD_MIN: f64 = 8e-4;

    /// ピボット閾値の 1 段の引き上げ倍率 (`0.25 -> 0.5` で上限に達する)。
    pub(crate) const PIVOT_THRESHOLD_FACTOR: f64 = 2.0;

    /// 消去開始前の次数が `m` のこの割合を超える列を「初期稠密」とみなす
    /// (`find_best_pivot` の稠密列回避、および境界付き分解の境界列検出)。
    pub(crate) const DENSE_COL_FRACTION: f64 = 0.5;

    /// 1 回の `find_best_pivot` が調べる候補列数の上限 (ピボットが見つかっている
    /// 場合のみ適用。`ENOMOTO_PIVOT_SEARCH_LIMIT` で上書き、`0` で無制限)。
    /// 最悪ケースの保険として大きめの値 (HiGHS は 8)。2026-09 に 256 から 64 に再調整
    /// ([`super::slope_intercept_dual::XB_CHECK_CADENCE`] 参照)。
    pub(crate) const PIVOT_SEARCH_LIMIT: usize = 64;

    /// `find_best_pivot` の行探索 (`ENOMOTO_PIVOT_ROW_SEARCH`) で走査する最大の行次数の
    /// 既定値。`0` = 行探索なし (経路が変わるため既定 off)。
    pub(crate) const PIVOT_ROW_SEARCH_MAX_DEGREE: usize = 0;

    // ---- 活性部分行列の格納 (KernelMatrix) ----

    /// この長さ以下の行ランは二分探索でなく線形探索で列を探す (`KernelMatrix::row_get`)。
    pub(crate) const KERNEL_LINEAR_SCAN_MAX: usize = 16;

    /// `KernelMatrix::new` がバッファ容量を予約する際の入力非ゼロ数に対する倍率
    /// (容量 = `KERNEL_RESERVE_MULT * nnz + KERNEL_RESERVE_EXTRA`)。
    pub(crate) const KERNEL_RESERVE_MULT: usize = 2;

    /// `KernelMatrix::new` の容量予約に加える固定の余裕。
    pub(crate) const KERNEL_RESERVE_EXTRA: usize = 64;

    /// 行・列ランを再配置するときの最小容量。
    pub(crate) const KERNEL_MIN_RUN_CAP: usize = 4;


    /// スレッドごとに保持しておく再利用バケット配列の最大個数 (`BUCKET_POOL`)。
    pub(crate) const BUCKET_POOL_MAX: usize = 4;

    // ---- 分解経路の振り分け ----

    /// 入力の非ゼロ数が `m^2` のこの割合を超えたら Markowitz をやめ、`faer` の
    /// 稠密部分ピボット LU (`factorize_dense_faer`) で分解する。
    pub(crate) const DENSE_INPUT_FRACTION: f64 = 0.25;

    /// 境界付き分解 (`factorize_bordered`) を試す境界列数 `k` の上限 (`k / m`)。
    pub(crate) const BORDER_MAX_FRACTION: f64 = 0.4;

    /// 境界付き分解を試す境界列数 `k` の絶対上限 (`k x k` 稠密分解のコストを
    /// 抑える防御的な上限。通常は `BORDER_MAX_FRACTION` が効く)。
    pub(crate) const BORDER_MAX_COUNT: usize = 3000;

    /// B3 (`ENOMOTO_LU_DENSE_SWITCH`): 活性部分行列の密度が残り `k^2` のこの割合に
    /// 達したら残りを稠密分解に切り替える。既定 `0.0` = 無効 (経路が変わるため)。
    pub(crate) const DENSE_SWITCH_FRACTION: f64 = 0.0;

    /// B3: 稠密切替を検討する残り行数の下限 (`ENOMOTO_LU_DENSE_SWITCH_MIN`)。
    pub(crate) const DENSE_SWITCH_MIN_ROWS: usize = 64;

    /// B3: 稠密切替の密度判定を行う消去ステップの間隔。
    pub(crate) const DENSE_SWITCH_CHECK_INTERVAL: usize = 16;

    // ---- ピボット順の再利用 (factorize_reusing) ----

    /// 再利用分解が作る因子の非ゼロ数が、最後の通常分解の `L`+`U` 非ゼロ数の
    /// この倍数を超えたら再利用を諦める (`ENOMOTO_REUSE_FILL_LIMIT` で上書き可)。
    pub(crate) const REBUILD_FILL_LIMIT: f64 = 1.25;

    /// 再利用分解で、列の残り部分行列の最大絶対値がこれ未満なら特異とみなして
    /// 諦める (質の判定ではなく「ピボットが無い」判定)。
    pub(crate) const REBUILD_MIN_PIVOT: f64 = 1e-12;

    /// 再利用棄却後の指数バックオフのシフト量の上限 (シフト自体のオーバーフロー防止)。
    pub(crate) const REUSE_BACKOFF_SHIFT_CAP: u32 = 5;

    /// 再利用棄却後に見送る再分解回数の上限。
    pub(crate) const REUSE_MAX_BACKOFF: u32 = 16;

    // ---- eta と FTRAN/BTRAN の疎/密切替 ----

    /// eta (`U` の列 eta・`R` の行 eta) の非対角 fill が `m` のこの割合を超えたら
    /// 密形式で格納する (実データでの調整はまだ)。
    pub(crate) const DENSE_ETA_FRACTION: f64 = 0.4;

    /// FTRAN の右辺の非ゼロ数が `m` のこの割合を超えたら、Gilbert-Peierls 疎経路を
    /// やめて密経路を使う (`FtLu::should_use_dense_solve`)。
    pub(crate) const DENSE_RHS_FRACTION: f64 = 0.4;

    /// `FtranDensity` の移動平均で最新の観測に与える重み (HiGHS の
    /// `kRunningAverageMultiplier`)。
    pub(crate) const DENSITY_AVERAGE_MULTIPLIER: f64 = 0.05;

    /// FTRAN チャネルの結果密度の移動平均がこの割合を超えたら、右辺が疎でも
    /// 密経路を使う (`ENOMOTO_EXPECTED_DENSITY_GATE` で上書き、`>= 1.0` で無効)。
    pub(crate) const EXPECTED_DENSE_FRACTION: f64 = 0.35;

    /// BTRAN の `L^{-T}` 段で、入力 `w` の非ゼロが `m` のこの割合以下なら行優先
    /// スキャッタ形式、超えたら列優先ギャザー形式を使う
    /// (`ENOMOTO_BTRAN_L_SCATTER` で上書き、`0` でスキャッタ無効)。
    pub(crate) const BTRAN_L_SCATTER_FRACTION: f64 = 0.10;

    /// 超疎ピボット行 BTRAN (`UnitBtranWork`、stormG2 報告 §4 策2): 非ゼロになりうる位置が
    /// `m` のこの割合を超えたら、優先度付きキューをやめてその位置から全走査に切り替える
    /// (`ENOMOTO_T_BTRAN_HYPER_FRACTION`)。前回の結果がこれを超えていたら最初から全走査。
    /// 結果はどちらでもビット一致なので速度だけの閾値。
    pub(crate) const BTRAN_HYPER_FRACTION: f64 = 0.10;

    /// C5 超疎 `U` 段: DFS の到達スロット数が `m` のこの割合を超えたら諦めて通常の
    /// 全走査にする (HiGHS の `kHyperFtranU` は 0.10)。
    pub(crate) const U_HYPER_ABORT_FRACTION: f64 = 0.25;

    /// BTRAN 結果の非ゼロステップ記録 (`StepCapture`) を諦める非ゼロ率 (`m` に対する割合、
    /// `ENOMOTO_T_TAU_GP_FRACTION`)。これ以下なら続く `tau` FTRAN の `L` 段を GP で行う。
    pub(crate) const TAU_GP_FRACTION: f64 = 0.1;

    /// 追加策 R: 融合 FTRAN の `R` 段を疎に当てる (`FtLu::apply_r_sparse`) のは `R` eta がこの数以上
    /// あるときだけ (`ENOMOTO_T_R_SPARSE_MIN_ETAS`)。少ないうちは全 eta を順に当てる方が速い
    /// (Netlib sctap1 で常時疎にすると +5%)。結果はどちらでもビット一致。
    pub(crate) const R_SPARSE_MIN_ETAS: usize = 256;

    /// FTRAN/BTRAN の結果や新しい eta 要素を厳密な 0 とみなす絶対値の閾値
    /// (`ENOMOTO_TINY`)。既定 `0.0` = 切り捨てなし。
    pub(crate) const TINY_DROP: f64 = 0.0;

    // ---- CLOCK 再分解トリガ用の決定的 tick ----

    /// `FtLu::build_tick` の `m` 比例項の係数 (HiGHS の `buildSynthticTick` と同じ 80)。
    pub(crate) const TICK_BUILD_M_COEF: u64 = 80;

    /// `FtLu::build_tick` の `nnz(L+U)` 比例項の係数 (HiGHS と同じ 60)。
    pub(crate) const TICK_BUILD_LU_COEF: u64 = 60;

    /// `FtLu::build_tick` の消去積和回数項の係数 (S16、`0` = 項なし)。
    pub(crate) const TICK_BUILD_FLOP_COEF: u64 = 0;

    /// 求解段の tick 増分 (触れた非ゼロ数) に掛ける係数 (`1` = 素の非ゼロ数)。
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
