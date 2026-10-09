//! 有界変数つき改訂単体法の共通基盤。
//!
//! `solver::solve_lp` の既定エンジン (MIP の各分枝限定ノードでも使用) で、
//! 実際の求解は傾き・切片二段解法 [`slope_intercept_dual::solve_slope_intercept_dual`] が行う。
//! 前処理後の問題が独立な連結成分に分かれる場合は成分ごとに解く
//! ([`solve_std_form_decomposed`])。このモジュールが持つのは次の共通部品:
//! 標準形 [`StdForm`] の構築と前処理との接続、基底状態 [`Tableau`]、
//! 傾き・切片二段解法の仕上げ段階から引き継がれる主単体法 [`run_phase`]、
//! 双対最急辺 (DSE) の重み [`DseState`] など。傾き・切片二段解法が解を出せなかった場合は
//! [`Status::NotSolved`] を返す。
//!
//! ## 変数の上下限
//!
//! 構造変数の上下限は片側/両側とも無限でもよい。前処理で消せなかった無限の上下限は
//! そのまま `slope_intercept_dual` に渡され、そこで記号的 (M の係数) に扱われる。
//! [`Tableau`] と主単体法は、`slope_intercept_dual` が真の上下限内に収めた基底しか受け取らない。
//! スラック列 (各行に 1 本) は `<=`/`>=` 行では `[0, inf)` の片側無限なので、
//! 比率テスト/EXPAND は無限上限を扱う必要がある。
//!
//! ## 基底の表現
//!
//! Markowitz ピボットの疎 LU (`sparse_lu::LuFactors`) と Forrest-Tomlin 更新
//! (`sparse_lu::FtLu`、Huangfu & Hall ERGO-13-001 の定式化) で保持し、
//! 次のいずれかで再分解する:
//!
//!   1. [`FT_CHECK_INTERVAL`] 反復ごとに、真の基底残差 `‖A_B x_B - rhs‖` が
//!      [`FT_RESIDUAL_TOL`] を超えたとき
//!   2. 更新後のピボットが [`FT_MIN_PIVOT`] 未満のとき (`FtLu::try_update` 内で即時判定)
//!   3. (1) と同じ周期で、eta ファイルの fill (`FtLu::fill_count`) が
//!      [`FT_BUMP_LIMIT_FACTOR`] `* m` を超えたとき
//!   4. 更新回数が [`FT_MAX_UPDATES`] を超えたとき (無条件)
//!
//! ## 標準形
//!
//! 各制約行に専用のスラック列を付け、`M z = rhs`, `lo <= z <= hi`, `z = [x; s]` とする:
//!
//!   - `A x = b`  →  `A x + s = b`,  `s` は `[0, 0]` に固定
//!   - `G x <= h` →  `G x + s = h`,  `s` は `[0, inf)`
//!   - `G x >= h` は呼び出し側で `-G x <= -h` に正規化済みなので `<=` と同じ扱い
//!
//! これにより制約の向きによらず、各行のスラックを初期基底 (`B = I`) にできる。
//!
//! ## 主単体法: 2 段階法 + EXPAND 巡回回避 + 最急辺
//!
//! 第 1 段階は基底変数の上下限違反量の和を、毎反復作り直す合成目的関数
//! (`x_Bi` が下限未満なら `-1`、上限超過なら `+1`、実行可能なら `0`) で最小化し、
//! 違反している変数は「実行可能側へ戻る方向の上下限」だけでブロックされる
//! 修正比率テストを使う。第 2 段階は通常の有界変数主単体法。両段階とも
//! EXPAND 比率テスト (Gill, Murray, Saunders & Wright 1989) と主最急辺の
//! 入る変数選択 (Forrest & Goldfarb 1992) を共有する
//! ([`ExpandState`], [`SteepestEdgeState`])。
//!
//! ## 双対法
//!
//! 本体は `slope_intercept_dual` (DSE 価格付け・BFRT・被約費用の差分更新)。
//! 上の主単体法はその仕上げ段階の引き継ぎ先としてだけ使われる。
//!
//! ## 並列化の方針
//!
//! 前処理 (`crate::presolve`) は rayon で並列化する。単体法の反復内ループ
//! (chuzr、chuzc1 の候補走査、被約費用/DSE/最急辺の重み更新) は、rayon の
//! 呼び出しオーバーヘッドが本体の計算を上回るため逐次実行にしている。
//! 大幅に大きな問題を対象にする場合は再計測してから並列化すること。

use crate::presolve::{self, scaling};
use crate::sparse::{CscMat, CsrMat, csr_row_iter, sparse_axpy_dense, sparse_dot_dense};
use crate::types::{ConstraintRow, Objective, RowSense, Sense, Status, VariableData};
use crate::params::simplex::{COST_PERTURB_BASE, COST_PERTURB_BOXED_FRACTION, COST_PERTURB_ZERO_COST_SCALE, COST_PERTURB_FEW_BOXED_COST_CAP, COST_PERTURB_LARGE_COST, EXPAND_DELTA_0, EXPAND_DELTA_F, EXPAND_K, EXPAND_TAU, FT_BUMP_LIMIT_FACTOR, FT_CHECK_INTERVAL, FT_MAX_UPDATES, FT_MIN_PIVOT, FT_RESIDUAL_TOL, MAX_ITERS_CEILING, MAX_ITERS_FLOOR, MAX_ITERS_SCALE, PARALLEL_COMPONENT_MIN_VARS, PARTIAL_PRICING_GROUPS, PARTIAL_PRICING_THRESHOLD, PRESOLVE_ROUNDS, PRIMAL_HARRIS_TOL, PRIMAL_STALL_LIMIT_MIN, PRIMAL_STALL_LIMIT_PER_ROW, PROPAGATION_PASSES, RACE_MIN_ROWS, RAYON_SIZE_THRESHOLD, XO_SERIAL_NNZ, XO_SPLIT_MIN_VARS, ROWSINGLETON_COLSINGLETON_INNER_ROUNDS, RUIZ_ITERS, STALL_PROGRESS_EPS, STEEPEST_EDGE_FLOOR, TOL, UPDATE_VERIFY_TOL};

/// Markowitz ピボットの疎 LU と Forrest-Tomlin 更新。このファイル内では
/// ローカル変数名 `lu` (FtLu インスタンス) との衝突を避けるため `sparse_lu` の別名で参照する。
pub(crate) mod lu;
/// FTRAN/BTRAN の結果や eta 要素を 0 とみなす絶対値の閾値 (他モジュール向けの再公開)。
pub(crate) use lu::tiny_drop;
/// `lu` モジュールの別名 (ローカル変数 `lu` との衝突回避)。
use self::lu as sparse_lu;

/// 基底の求解の心臓部 (FTRAN・BTRAN・FT 更新・再分解の判定)。主単体法と双対単体法で共有する。
mod basis_kernel;
/// 傾き・切片二段解法 (実際の LP 求解本体)。
pub(crate) mod slope_intercept_dual;
/// 内点法 + クロスオーバー (Liu & Lu 2024) による求解 (`RootSolver::IpmCrossover`)。
mod crossover;
/// 傾き・切片双対二段解法と内点法 + クロスオーバーの同時実行 (`RootSolver::Auto`)。
mod race;
mod sifting;
mod dualize;
/// 分枝限定法から二段解法を使うためのラッパー (前処理なしの標準形を保持し、warm start で解く)。
pub(crate) mod mip_lp;
#[cfg(test)]
mod lp_bug_debug;

/// 単体法の各メインループの反復上限を問題サイズから決める。
///
/// `MAX_ITERS_SCALE * (m + n_total)` を [`MAX_ITERS_FLOOR`]..[`MAX_ITERS_CEILING`] に収めた値。
/// 有限停止の保証は Bland 規則 (`bland_mode`) が担い、これは 1 回の求解の実時間を
/// 抑えるための上限にすぎないので、必要反復数に対して十分な余裕を持たせてある。
fn max_iters_for(m: usize, n_total: usize) -> usize {
    (MAX_ITERS_SCALE * (m + n_total)).clamp(MAX_ITERS_FLOOR, MAX_ITERS_CEILING)
}

/// ピボット要素の整合性検査 (HiGHS `HEkkDualRow::updateVerify` 相当)。
///
/// `alpha_row` は PRICE で得た行方向の値 (`a_p[q]`)、`alpha_col` は FTRAN で得た
/// 列方向の値 (`alpha_buf[p]`) で、同じピボットの 2 通りの計算値でなければならない。
/// 両者の相対差が [`UPDATE_VERIFY_TOL`] 以下なら `true` (このピボットで続行)、
/// そうでなければ `false` (コミット前に再分解すべき)。ピボットの選び方には影響せず、
/// 再分解のタイミングだけを変える。`slope_intercept_dual` からも呼ばれる。
#[inline]
pub(super) fn pivot_values_agree(alpha_row: f64, alpha_col: f64) -> bool {
    let scale = alpha_row.abs().max(alpha_col.abs()).max(tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64));
    let rel = (alpha_row - alpha_col).abs() / scale;
    rel <= UPDATE_VERIFY_TOL
}

/// EXPAND 巡回回避の作業許容誤差とリセット周期の状態。
/// 1 回の `solve_lp` 中、第 1 段階と第 2 段階をまたいで引き継がれる。
struct ExpandState {
    /// 現在の作業実行可能性許容誤差 `delta` (毎反復 `EXPAND_TAU` ずつ増える)。
    delta: f64,
    /// 直近のリセットからの反復数 (`EXPAND_K` に達したらリセット)。
    iters_since_reset: usize,
}

impl ExpandState {
    /// `delta = EXPAND_DELTA_0` から始まる新しい状態を作る。
    fn new() -> Self {
        ExpandState { delta: EXPAND_DELTA_0, iters_since_reset: 0 }
    }
}

/// 主単体法の停滞検出と Bland 規則 (1977) への切り替え状態。
///
/// 目的関数への寄与 (`theta * dj`) が [`STALL_PROGRESS_EPS`] 未満のピボットが
/// `stall_limit` 回を超えて連続したら `bland_mode` を立て、以後その求解の終わりまで
/// Bland 規則 (最小添字) で入る/出る変数を選ぶ。第 1/第 2 段階をまたいで引き継ぐ。
struct PrimalStallState {
    /// 進展のないピボットの連続回数。
    stall_count: usize,
    /// Bland 規則モードに入ったか (一度立つと戻らない)。
    bland_mode: bool,
}

impl PrimalStallState {
    /// カウンタ 0・Bland モード無効の初期状態を作る。
    fn new() -> Self {
        PrimalStallState { stall_count: 0, bland_mode: false }
    }
}

/// 部分価格付けの第 1 パスの標本に列 `j` が含まれるかを決定的に判定する。
///
/// およそ [`PARTIAL_PRICING_GROUPS`] 列に 1 列が標本に入る。`(seed, iter, j)` の純関数なので、
/// 同じ呼び出しで「標本内か」(第 1 パス) と「標本外か」(第 2 パス、否定) の両方に使え、
/// 実行ごとの再現性もある。`seed` は問題ごと、`iter` は反復ごとに標本を変える。
/// ビット攪拌は splitmix64 の finalizer。
#[inline]
fn partial_pricing_sampled(seed: u64, iter: u64, j: usize) -> bool {
    let mut z = seed ^ iter.wrapping_mul(0x9E3779B97F4A7C15) ^ (j as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58476D1CE4E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;
    z % PARTIAL_PRICING_GROUPS == 0
}

/// 主単体法の最急辺 (steepest edge) 重み (Forrest & Goldfarb 1992)。
///
/// 非基底列 `j` について `gamma[j] = ||B^-1 A_j||^2`。入る変数は Dantzig の
/// `max |d_j|` ではなく `max d_j^2 / gamma_j` で選ぶ。更新式は
/// `B_new^-1 = E^-1 B^-1` への Sherman-Morrison から導出したもので、
/// テスト `steepest_edge_weights_match_brute_force_recompute` で検証している。
struct SteepestEdgeState {
    /// 列ごとの重み `gamma[j]` (長さ `n_total`、基底列の値は未使用)。
    gamma: Vec<f64>,
}

impl SteepestEdgeState {
    /// 初期基底 (符号付き単位行列) に対する重みを作る。構造列は `gamma[j] = ||A_j||^2`、
    /// 初期に基底であるスラック列は仮の値 1 (非基底になる時に `run_phase` 側で設定される)。
    fn new(std: &StdForm) -> Self {
        let mut gamma = vec![1.0; std.n_total];
        let n_orig = std.n_total - std.n_rows;
        for j in 0..n_orig {
            let norm_sq: f64 = std.cols.col(j).iter().map(|&(_, v)| v * v).sum();
            gamma[j] = norm_sq.max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
        }
        SteepestEdgeState { gamma }
    }

    /// ピボット後の Forrest-Goldfarb 重み更新。
    ///
    /// 引数はすべてピボット前の基底に対する値: `rho` = `B^-1` の第 `r` 行、
    /// `w` = `B^-T alpha`、`gamma_t_old` = 入る列の旧重み、`pivot = alpha[r]`。
    /// ただし `t` の基底/非基底状態はピボット後のものを渡すこと。
    fn update_after_pivot(&mut self, t: &Tableau, std: &StdForm, rho: &[f64], w: &[f64], gamma_t_old: f64, pivot: f64) {
        // 列ごとに独立な更新 (並列化可能だが呼び出しコストの都合で逐次)。
        for (j, gamma_j) in self.gamma.iter_mut().enumerate() {
            // 基底列と固定列 (入る変数に選ばれない) は更新不要。
            if t.nb_status[j].is_none() || std.lb[j] == std.ub[j] {
                continue;
            }
            let col = t.column_sparse(j);
            let pivot_sj: f64 = col.iter().map(|&(i, v)| v * rho[i]).sum(); // ピボット行の j 成分 (rho・A_j)
            let tau_j: f64 = col.iter().map(|&(i, v)| v * w[i]).sum();
            let beta_j = pivot_sj / pivot;
            *gamma_j = (*gamma_j + beta_j * beta_j * (1.0 + gamma_t_old) - 2.0 * beta_j * tau_j).max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
        }
    }
}

/// 非基底変数がどの値に置かれているか。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NbStatus {
    /// 下限に置かれている。
    Lower,
    /// 上限に置かれている。
    Upper,
    /// 値 0 に置かれた自由列 (論文の状態 `Z`, 3.1 節)。`lb == -inf` かつ `ub == +inf` の
    /// 列に限り、`slope_intercept_dual` だけが作る (費用 0 の自由列のクラッシュ、cleanup の場合 (A))。
    /// 被約費用がちょうど 0 のとき双対実行可能で、どちら向きにも入れ、フリップはしない。
    /// [`run_phase`] も引き継ぎで受け取りうるので、すべての match で扱う。
    Zero,
}

/// LP 求解の結果 (状態と、最適なら元の変数空間での解)。
pub struct SimplexResult {
    /// 求解状態。
    pub status: Status,
    /// 解ベクトル (`Optimal` のときのみ `Some`)。
    pub x: Option<Vec<f64>>,
}

/// 単体法が扱う標準形 `[A x + s = b]`, `lb <= z <= ub` (モジュール文書参照)。
///
/// 列 `0..n_total - n_rows` が構造変数、残り `n_rows` 列が各行のスラック。
/// 求解中は変更されない (変わるのは [`Tableau`] の基底状態と `x` だけ) ので、
/// 行形式 `rows` と列形式 `cols` がずれることはない。
#[derive(Clone)]
struct StdForm {
    /// 全列数 (構造変数 + スラック)。
    n_total: usize,
    /// 行数 (= スラック列数)。
    n_rows: usize,
    /// 最小化形の目的関数係数 (長さ `n_total`、スラックは 0)。
    c: Vec<f64>,
    /// 制約行列の行形式 (各行は列番号昇順)。
    rows: CsrMat,
    /// `rows` の転置 (列形式)。`cols.col(j)` は列 `j` の `(行, 値)`。
    cols: CscMat,
    /// 右辺 `b`。
    b: Vec<f64>,
    /// 各列の下限。
    lb: Vec<f64>,
    /// 各列の上限。
    ub: Vec<f64>,
}

/// 行リスト `rows` から [`StdForm`] 用の行形式/列形式の行列対を作る。
///
/// 前提: 各行は列番号の厳密な昇順 (`debug_assert` で確認)。列形式からの走査が
/// 行形式と同じ要素順を再現し、LU のピボット順や残差の加算順がビット単位で
/// 一致することがこれに依存している。
fn freeze_std_matrices(rows: &[Vec<(usize, f64)>], n_total: usize) -> (CsrMat, CscMat) {
    debug_assert!(
        rows.iter().all(|r| r.windows(2).all(|w| w[0].0 < w[1].0)),
        "StdForm rows must be strictly column-ascending"
    );
    (CsrMat::from_rows(rows, n_total), CscMat::from_rows(rows, n_total))
}

/// モデルの変数/目的/制約から前処理なしで直接 [`StdForm`] を作る (テスト用)。
/// 各行に係数 +1 (`>=` 行は -1) のスラックを 1 本付ける。
fn build_std_form(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow]) -> StdForm {
    let n = variables.len();
    let n_rows = constraints.len();
    let n_total = n + n_rows;

    let mut c = vec![0.0; n_total];
    let sign = match objective.sense { // 最大化は符号反転して最小化にする
        Sense::Minimize => 1.0,
        Sense::Maximize => -1.0,
    };
    for (&j, &v) in objective.expr.coeffs.iter() {
        c[j] = sign * v;
    }

    let mut lb = vec![0.0; n_total];
    let mut ub = vec![0.0; n_total];
    for (j, v) in variables.iter().enumerate() {
        lb[j] = v.lb;
        ub[j] = v.ub;
    }

    let mut rows = Vec::with_capacity(n_rows);
    let mut b = Vec::with_capacity(n_rows);
    for (i, row) in constraints.iter().enumerate() {
        let slack = n + i;
        let mut r: Vec<(usize, f64)> = row.expr.coeffs.iter().map(|(&j, &v)| (j, v)).collect();
        let rhs = row.rhs - row.expr.constant;
        match row.sense {
            RowSense::Eq => {
                lb[slack] = 0.0;
                ub[slack] = 0.0;
                r.push((slack, 1.0));
                b.push(rhs);
            }
            RowSense::Le => {
                lb[slack] = 0.0;
                ub[slack] = f64::INFINITY;
                r.push((slack, 1.0));
                b.push(rhs);
            }
            RowSense::Ge => {
                lb[slack] = 0.0;
                ub[slack] = f64::INFINITY;
                r.push((slack, -1.0));
                b.push(rhs);
            }
        }
        rows.push(r);
    }

    let (rows, cols) = freeze_std_matrices(&rows, n_total);
    StdForm { n_total, n_rows, c, rows, cols, b, lb, ub }
}

/// [`build_std_form_presolved`] の戻り値: 前処理・圧縮済みの [`StdForm`] と、
/// 求解結果を元の `variables.len()` 次元の `x` に戻すための情報。
///
/// 固定変数 (代入消去されたものも含む) は `std` から完全に除かれている。
/// 圧縮後の構造列 `nj` の元の値は `sign[nj] * (x_kept[nj] + shift[nj])` で復元する
/// (まだスケーリング空間)。`orig_of_kept.len() + fixed_values.len() == variables.len()`。
struct PresolvedForm {
    /// 前処理・圧縮済みの標準形。
    std: StdForm,
    /// Ruiz スケーリングの係数 (解の逆スケーリングに使う)。
    scaling: scaling::Scaling,
    /// 消去系の前処理ステップの時系列ログ (1 本のまま [`unscale_result`] が逆順に適用する)。
    postsolve_log: Vec<presolve::PostsolveStep>,
    /// 圧縮後の構造列 `nj` に対応する元の変数番号。
    orig_of_kept: Vec<usize>,
    /// 列 `nj` の符号。下側のみ無限で反転 (`x = ub - y`) した列は `-1.0`、それ以外は `1.0`。
    /// 行係数・費用にも同じ符号が掛けてある。
    sign: Vec<f64>,
    /// `std` から除いた変数の `(元の番号, 値)`。代入消去された変数の値は仮の 0 で、後で上書きされる。
    fixed_values: Vec<(usize, f64)>,
    /// 列 `nj` の座標の平行移動量 (有限の上下限を 0 に寄せた量、自由列は 0)。
    shift: Vec<f64>,
}

/// 共通前処理 (`presolve::run_extended`: Ruiz スケーリング、冗長行除去、制約伝播、
/// 各種消去) を実行し、その出力から [`PresolvedForm`] を作る。
///
/// `A` の行は `Eq` 行、`G` の多変数行は `Le` 行になり、いずれも行ごとに 1 本のスラックを持つ。
/// `G` の単一変数行は `lb`/`ub` として取り込まれる。続けて、生き残った構造列の座標を
/// 平行移動 (と必要なら反転) して有限の上下限を 0 に揃え、固定列を除いて列番号を詰める。
/// 代入は全てスケーリング後の座標で記録される。
///
/// `allow_unbounded_verdict` が真なら、前処理が改善方向の半直線を見つけた時点で
/// `InfeasibleOrUnbounded` を返してよい。前処理だけで実行不能が証明されれば
/// `Err(Status::Infeasible)` を返す。
fn build_std_form_presolved(
    variables: &[VariableData],
    objective: &Objective,
    constraints: &[ConstraintRow],
    allow_unbounded_verdict: bool,
) -> Result<PresolvedForm, Status> {
    let n = variables.len();
    let sign = match objective.sense { // 最大化は符号反転して最小化にする
        Sense::Minimize => 1.0,
        Sense::Maximize => -1.0,
    };
    let mut c0 = vec![0.0; n]; // 元の変数空間での最小化形費用
    for (&j, &v) in objective.expr.coeffs.iter() {
        c0[j] = sign * v;
    }

    let (a, b, g, h) = presolve::build_a_g(variables, constraints);

    let pre = presolve::run_extended(
        n,
        &a,
        &b,
        &g,
        &h,
        &c0,
        tunable!("ENOMOTO_T_RUIZ_ITERS", RUIZ_ITERS, usize),
        tunable!("ENOMOTO_T_PROPAGATION_PASSES", PROPAGATION_PASSES, usize),
        tunable!("ENOMOTO_T_PRESOLVE_ROUNDS", PRESOLVE_ROUNDS, usize),
        tunable!("ENOMOTO_T_INNER_ROUNDS", ROWSINGLETON_COLSINGLETON_INNER_ROUNDS, usize),
        allow_unbounded_verdict,
    );
    if env_str!("ENOMOTO_DEBUG_PRESOLVE_INFEAS").is_some() {
        eprintln!("DEBUG_PRESOLVE: infeasible={} unbounded={}", pre.infeasible, pre.unbounded);
    }
    if pre.infeasible {
        return Err(Status::Infeasible);
    }
    if pre.unbounded {
        // 前処理 (freevar) が改善方向の半直線を見つけた: 有限最適解はないが、
        // 残りが実行可能かは未確定なので InfeasibleOrUnbounded。
        return Err(Status::InfeasibleOrUnbounded);
    }

    // 伝播で分離済みの箱型上下限と多変数不等式行をそのまま使う。
    let (mut lb, mut ub, g_rows, g_rhs) = (pre.lb, pre.ub, pre.real_rows, pre.real_rhs);
    // 代入消去された変数は上下限が無限のまま残っているので、求解に渡す前に
    // 任意の有限点 0 に固定する (値は後で `Substitution::value` で復元される)。
    for step in &pre.postsolve_log {
        if let presolve::PostsolveStep::Sub(sub) = step {
            lb[sub.var] = 0.0;
            ub[sub.var] = 0.0;
        }
    }

    // 座標の平行移動: 固定されていない構造列を、有限な方の上下限が 0 になるよう
    // `x_j = x'_j + shift[j]` と移す (両側有限なら下限へ、片側有限ならその側へ、
    // 自由列は移動なし)。上限だけ有限で下側が無限の列は先に `x_j = ub[j] - y_j` と
    // 反転して `[0, +inf)` 型にそろえ、係数の符号反転は `reflect_sign` に記録する。
    // 行の右辺は下の行構築ループで、解は [`unscale_result`] で対応して補正する。
    let mut reflect_sign = vec![1.0; n]; // 反転した列は -1.0
    let mut shift = vec![0.0; n]; // 各列の平行移動量
    for j in 0..n {
        if lb[j] == ub[j] {
            continue;
        }
        if lb[j] == f64::NEG_INFINITY && ub[j].is_finite() {
            let u = ub[j];
            reflect_sign[j] = -1.0;
            lb[j] = -u;
            ub[j] = f64::INFINITY;
        }
        let s = if lb[j].is_finite() {
            lb[j]
        } else if ub[j].is_finite() {
            ub[j]
        } else {
            0.0
        };
        shift[j] = s;
        lb[j] -= s;
        ub[j] -= s;
    }

    // まだ無限の上下限を持つ構造列はそのまま残し、`slope_intercept_dual` が記号的に扱う。
    if env_str!("ENOMOTO_DEBUG_UNBOUNDED_VARS").is_some() {
        let count = (0..n).filter(|&j| lb[j] == f64::NEG_INFINITY || ub[j] == f64::INFINITY).count();
        if count > 0 {
            eprintln!("PRESOLVE: {count} structural column(s) still have a genuine infinite bound");
        }
    }

    // 列の圧縮: `lb[j] < ub[j]` の構造列だけに連番の列番号を振り、固定列は
    // 求解の列空間から外す (その寄与は下で各行の右辺に畳み込む)。両側無限の
    // 自由列も通常の列と同じく 1 列として残す (`slope_intercept_dual` が両側を記号的に扱う)。
    let mut new_index: Vec<Option<usize>> = vec![None; n]; // 元の列 j → 圧縮後の列番号 (固定列は None)
    let mut orig_of_kept: Vec<usize> = Vec::new();
    let mut sign: Vec<f64> = Vec::new();
    let mut slot_lb: Vec<f64> = Vec::new(); // 圧縮後の各列の下限
    let mut slot_ub: Vec<f64> = Vec::new(); // 圧縮後の各列の上限
    for j in 0..n {
        if lb[j] == ub[j] {
            continue;
        }
        let nj = orig_of_kept.len();
        orig_of_kept.push(j);
        sign.push(reflect_sign[j]);
        slot_lb.push(lb[j]);
        slot_ub.push(ub[j]);
        new_index[j] = Some(nj);
    }
    let n_kept = orig_of_kept.len(); // 残った構造列の数
    let fixed_values: Vec<(usize, f64)> = (0..n).filter(|&j| new_index[j].is_none()).map(|j| (j, lb[j])).collect();

    let n_eq = pre.a.nrows(); // 等式行の数
    let n_le = g_rows.len(); // `<=` 行の数
    let n_rows = n_eq + n_le;
    let n_total = n_kept + n_rows;

    let mut c = vec![0.0; n_total];
    for (nj, &j) in orig_of_kept.iter().enumerate() {
        c[nj] = sign[nj] * pre.c[j];
    }

    let mut new_lb = vec![0.0; n_total];
    let mut new_ub = vec![0.0; n_total];
    new_lb[..n_kept].copy_from_slice(&slot_lb);
    new_ub[..n_kept].copy_from_slice(&slot_ub);

    let mut b_out = Vec::with_capacity(n_rows);
    // 行は 1 本の平坦な CSR バッファに直接書き込む (行 `k` = `entries[offsets[k]..offsets[k + 1]]`)。
    // 固定列の寄与は右辺へ、平行移動量は `rhs -= a * sign * shift` で右辺へ畳み込む。
    let nnz_bound = pre.a.as_ref().compute_nnz() + g_rows.iter().map(|r| r.len()).sum::<usize>() + n_rows;
    let mut offsets: Vec<usize> = Vec::with_capacity(n_rows + 1);
    offsets.push(0);
    let mut entries: Vec<(usize, f64)> = Vec::with_capacity(nnz_bound);

    for i in 0..n_eq {
        let slack = n_kept + i;
        let mut rhs_i = pre.b[i];
        for (j, v) in csr_row_iter(&pre.a, i) {
            match new_index[j] {
                Some(nj) => {
                    entries.push((nj, v * sign[nj]));
                    rhs_i -= v * sign[nj] * shift[j];
                }
                None => rhs_i -= v * lb[j],
            }
        }
        new_lb[slack] = 0.0;
        new_ub[slack] = 0.0;
        entries.push((slack, 1.0));
        offsets.push(entries.len());
        b_out.push(rhs_i);
    }
    for (k, row) in g_rows.into_iter().enumerate() {
        let slack = n_kept + n_eq + k;
        let mut rhs_k = g_rhs[k];
        for (j, v) in row {
            match new_index[j] {
                Some(nj) => {
                    entries.push((nj, v * sign[nj]));
                    rhs_k -= v * sign[nj] * shift[j];
                }
                None => rhs_k -= v * lb[j],
            }
        }
        new_lb[slack] = 0.0;
        new_ub[slack] = f64::INFINITY;
        entries.push((slack, 1.0));
        offsets.push(entries.len());
        b_out.push(rhs_k);
    }

    debug_assert!(
        offsets.windows(2).all(|w| entries[w[0]..w[1]].windows(2).all(|e| e[0].0 < e[1].0)),
        "StdForm rows must be strictly column-ascending"
    );
    let rows = CsrMat::from_flat(n_total, offsets, entries);
    let cols = rows.to_csc();
    let shift_of_kept: Vec<f64> = orig_of_kept.iter().map(|&j| shift[j]).collect();
    Ok(PresolvedForm {
        std: StdForm { n_total, n_rows, c, rows, cols, b: b_out, lb: new_lb, ub: new_ub },
        scaling: pre.scaling,
        postsolve_log: pre.postsolve_log,
        orig_of_kept,
        sign,
        fixed_values,
        shift: shift_of_kept,
    })
}

/// 圧縮・スケーリング空間の求解結果を元の変数空間に戻す。
///
/// 手順: (1) 圧縮列を `sign * (x_kept + shift)` で元の番号に展開し、固定値を置く。
/// (2) `postsolve_log` を逆順に 1 回走査して代入消去/平行列統合を元に戻す
/// (代入はスケーリング空間で記録されているので、逆スケーリングより前に行う)。
/// (3) `scaling::unscale_x` で逆スケーリングする。`Optimal` 以外は `x` なしでそのまま返す。
/// `sc` はスケーリング係数、`n` は元の変数の数 (`variables.len()`)。
fn unscale_result(
    result: SimplexResult,
    sc: &scaling::Scaling,
    postsolve_log: &[presolve::PostsolveStep],
    orig_of_kept: &[usize],
    sign: &[f64],
    fixed_values: &[(usize, f64)],
    shift: &[f64],
    n: usize,
) -> SimplexResult {
    match result.status {
        Status::Optimal => {
            let x_kept = result.x.unwrap(); // 圧縮・平行移動後の座標での解
            // 圧縮列を元の番号に展開し、固定値も先に置いておく (代入式が固定変数を参照しうるため)。
            let mut x = vec![0.0; n];
            for (nj, &j) in orig_of_kept.iter().enumerate() {
                x[j] = sign[nj] * (x_kept[nj] + shift[nj]);
            }
            for &(j, v) in fixed_values {
                x[j] = v;
            }
            // 共通の時系列ログを逆順に 1 回だけ適用する (種類別に分けると順序依存が壊れる)。
            for step in postsolve_log.iter().rev() {
                step.apply(&mut x)
            }
            let x = scaling::unscale_x(sc, &x);
            SimplexResult { status: Status::Optimal, x: Some(x) }
        }
        other => SimplexResult { status: other, x: None },
    }
}

/// 単体法の基底状態: どの列が基底か、非基底列がどの上下限にあるか、全列の現在値。
struct Tableau<'a> {
    /// 対象の標準形。
    std: &'a StdForm,
    /// `basis[i]` = 基底の第 `i` 位置 (B の第 i 列) にある変数。
    basis: Vec<usize>,
    /// `basis` の逆写像: 変数 `var` が基底なら `Some(位置)`、非基底なら `None`。
    basis_pos: Vec<Option<usize>>,
    /// 非基底列の状態 (基底列は `None`)。
    nb_status: Vec<Option<NbStatus>>,
    /// 全列 (構造 + スラック) の現在値。
    x: Vec<f64>,
}

impl<'a> Tableau<'a> {
    /// スラック基底 (`B = I`)、構造変数はすべて下限に置いた初期状態を作る (テスト用)。
    #[cfg(test)]
    fn new(std: &'a StdForm) -> Self {
        let n_total = std.n_total;
        let n_rows = std.n_rows;
        let mut nb_status = vec![None; n_total];
        let mut x = vec![0.0; n_total];

        // 構造変数は下限で非基底 (有限の上下限を仮定)。
        for j in 0..(n_total - n_rows) {
            x[j] = std.lb[j];
            nb_status[j] = Some(NbStatus::Lower);
        }
        // スラックが基底 (各行 1 本、B = I)。
        let basis: Vec<usize> = (0..n_rows).map(|i| (n_total - n_rows) + i).collect();
        let mut basis_pos = vec![None; n_total];
        for (col, &var) in basis.iter().enumerate() {
            basis_pos[var] = Some(col);
        }

        Tableau { std, basis, basis_pos, nb_status, x }
    }

    /// 構造変数 (スラック以外) の列数。
    fn n_structural(&self) -> usize {
        self.std.n_total - self.std.n_rows
    }

    /// 制約行列の列 `j` を密ベクトルで返す (テスト用、実運用は [`Self::column_into`])。
    #[cfg(test)]
    fn column(&self, j: usize) -> Vec<f64> {
        let m = self.std.n_rows;
        let mut col = vec![0.0; m];
        self.column_into(j, &mut col);
        col
    }

    /// 列 `j` を呼び出し側のバッファ `out` (長さ `n_rows`) に密形式で書き込む。
    fn column_into(&self, j: usize, out: &mut [f64]) {
        for v in out.iter_mut() {
            *v = 0.0;
        }
        for &(i, v) in self.std.cols.col(j) {
            out[i] = v;
        }
    }

    /// 列 `j` の疎な `(行, 値)` 列をそのまま返す (内積だけ必要な走査用)。
    fn column_sparse(&self, j: usize) -> &[(usize, f64)] {
        self.std.cols.col(j)
    }

    /// 非基底の値から `B x_B = b - N x_N` を解いて基底変数の値を作り直す。
    /// 残差検査に使えるよう、計算した右辺 `b - N x_N` を返す。
    fn recompute_basics(&mut self, lu: &sparse_lu::FtLu) -> Vec<f64> {
        let rhs = self.compute_rhs();
        self.resync_basics(lu, &rhs);
        rhs
    }

    /// 現在の非基底値から右辺 `b - N x_N` を計算する (`x_B` は変更しない)。
    /// 値が 0 の非基底列は読み飛ばす (平行移動で 0 にある列が多い)。
    fn compute_rhs(&self) -> Vec<f64> {
        let mut rhs = self.std.b.clone();
        for j in 0..self.std.n_total {
            if self.nb_status[j].is_none() {
                continue;
            }
            let xj = self.x[j];
            if xj == 0.0 {
                continue;
            }
            for &(i, v) in self.std.cols.col(j) {
                rhs[i] -= v * xj;
            }
        }
        rhs
    }

    /// 計算済みの右辺 `rhs` で `B x_B = rhs` を `lu` で解き、`x_B` を上書きする。
    fn resync_basics(&mut self, lu: &sparse_lu::FtLu, rhs: &[f64]) {
        let sol = lu.solve(rhs);
        for i in 0..self.std.n_rows {
            self.x[self.basis[i]] = sol[i];
        }
    }

    /// 真の基底行列 (LU ではなく元の列) による残差 `‖A_B x_B - rhs‖`。
    /// 再分解トリガ (1) の数値ドリフト検査に使う。
    fn basis_residual_norm(&self, rhs: &[f64]) -> f64 {
        let m = self.std.n_rows;
        // `A_B x_B` を基底列ごとに加算する。
        let mut val = vec![0.0; m];
        for j in 0..self.std.n_total {
            if self.nb_status[j].is_some() {
                continue;
            }
            sparse_axpy_dense(self.x[j], self.std.cols.col(j), &mut val);
        }
        let mut resid_sq = 0.0;
        for i in 0..m {
            let r = val[i] - rhs[i];
            resid_sq += r * r;
        }
        resid_sq.sqrt()
    }

    /// EXPAND のリセット (Gill et al. 1989 §4.3): 状態が指す上下限から `EXPAND_DELTA_F` 以内にある
    /// 非基底変数をちょうどその値に戻す。基底変数の値は呼び出し側の次の
    /// `recompute_basics` で更新される。
    fn expand_reset_nonbasics(&mut self) {
        for j in 0..self.std.n_total {
            match self.nb_status[j] {
                Some(NbStatus::Lower) => {
                    if (self.x[j] - self.std.lb[j]).abs() < EXPAND_DELTA_F {
                        self.x[j] = self.std.lb[j];
                    }
                }
                Some(NbStatus::Upper) => {
                    if (self.x[j] - self.std.ub[j]).abs() < EXPAND_DELTA_F {
                        self.x[j] = self.std.ub[j];
                    }
                }
                Some(NbStatus::Zero) => {
                    if self.x[j].abs() < EXPAND_DELTA_F {
                        self.x[j] = 0.0;
                    }
                }
                None => {}
            }
        }
    }

}

/// [`perturb_costs`] の基準の大きさ `COST_PERTURB_BASE * (減衰・頭打ち後の最大費用)`。
/// 双対単体法の費用シフト (作業 #8、`slope_intercept_dual`) も同じ大きさを使う。
fn cost_perturb_base(std: &StdForm) -> f64 {
    let n = std.n_total;
    let mut max_abs_cost = std.c.iter().fold(0.0f64, |acc, &c| acc.max(c.abs()));
    if max_abs_cost > COST_PERTURB_LARGE_COST {
        max_abs_cost = max_abs_cost.sqrt().sqrt();
    }
    let boxed = (0..n).filter(|&j| (std.ub[j] - std.lb[j]).is_finite()).count(); // 箱型 (両側有限) 列の数
    if (boxed as f64) < COST_PERTURB_BOXED_FRACTION * (n.max(1) as f64) {
        max_abs_cost = max_abs_cost.min(COST_PERTURB_FEW_BOXED_COST_CAP);
    }
    // 費用が全部 0(実行可能性判定問題)だと基準が 0 になり摂動が消え、被約費用がすべて 0 のまま
    // 比率テストが全候補同点になって退化ピボットを重ねる(klein2: 1740 反復、99.9% 退化)。
    // その場合は単位費用と同じ大きさの基準を使う(klein2 は 214 反復)。
    if max_abs_cost == 0.0 {
        max_abs_cost = COST_PERTURB_ZERO_COST_SCALE;
    }
    COST_PERTURB_BASE * max_abs_cost
}

/// 列番号 `j` の splitmix64 風ハッシュから作る [0, 1) の擬似乱数 (費用摂動・費用シフトの列ごとの揺らぎ)。
fn perturb_random(j: usize) -> f64 {
    let mut h = (j as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    (h >> 40) as f64 / (1u64 << 24) as f64
}

thread_local! {
    /// 作業 #8 対処 5: 真なら [`perturb_costs`] は試験用の縮小 (`ENOMOTO_T_PERTURB_*FACTOR`) を無視して既定の大きさで
    /// 摂動する (双対単体法が壊れた基底から解き直すとき)。
    static FULL_COST_PERTURBATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// [`FULL_COST_PERTURBATION`] を設定する。
pub(crate) fn set_full_cost_perturbation(on: bool) {
    FULL_COST_PERTURBATION.with(|f| f.set(on));
}

/// 双対法の退化対策としての費用摂動 (HiGHS `HEkk::initialiseCost` と同じ方式)。
///
/// 摂動の大きさは最大費用の絶対値に比例 (大きすぎれば 4 乗根で減衰、箱型列が
/// ごく少なければ上限を設ける)。各列の摂動は `(1 + r) * (|c_j| + 1) * base`
/// (`r` は列番号のハッシュによる [0, 1) の擬似乱数) で、双対実行可能側を変えない向きに加える:
/// 固定列・自由列はそのまま、片側有限列は欠けている上下限から遠ざかる向き、
/// 箱型列は元の費用の符号の向き。摂動後の費用ベクトルを返す。
fn perturb_costs(std: &StdForm) -> Vec<f64> {
    let n = std.n_total;
    let base = cost_perturb_base(std); // 摂動の基準の大きさ
    // 試験用の縮小係数 (下のループ参照)。解き直しでは使わない。
    let (factor, zero_cost_factor) = if FULL_COST_PERTURBATION.with(|f| f.get()) {
        (1.0, 1.0)
    } else {
        (tunable!("ENOMOTO_T_PERTURB_FACTOR", 1.0, f64), tunable!("ENOMOTO_T_PERTURB_ZERO_COST_FACTOR", 1.0, f64))
    };

    let mut pc = std.c.clone(); // 摂動後の費用
    for j in 0..n {
        let lo = std.lb[j];
        let hi = std.ub[j];
        let free = !lo.is_finite() && !hi.is_finite();
        let fixed = lo == hi;
        if free || fixed {
            continue;
        }

        let r = perturb_random(j);
        let mut xpert = (1.0 + r) * (pc[j].abs() + 1.0) * base; // この列の摂動量
        // 作業 #8 対処 1: 列ごとの摂動は基準の大きさを割らない (摂動を縮める変更をするときもこの下限を守る。
        // 費用 0 の列の摂動を 0 にした案 E で pilot87 が誤って infeasible を返した)。
        debug_assert!(xpert >= base, "cost perturbation of column {j} fell below the floor");
        // 試験用 (作業 #8、既定は係数 1 = 無効): 摂動の大きさを変えて、摂動が小さい・無いときの正しさを試す
        // (`scripts/singular_stress.py` の z0/p0 などの設定)。`ENOMOTO_T_PERTURB_FACTOR`: 全列の摂動を係数倍
        // (0 で摂動なし)。`ENOMOTO_T_PERTURB_ZERO_COST_FACTOR`: 費用 0 の列だけ係数倍。
        xpert *= factor;
        if pc[j] == 0.0 {
            xpert *= zero_cost_factor;
        }
        if !hi.is_finite() {
            pc[j] += xpert;
        } else if !lo.is_finite() {
            pc[j] -= xpert;
        } else {
            pc[j] += if pc[j] >= 0.0 { xpert } else { -xpert };
        }
    }
    pc
}

/// 現在の基底を再分解する。特異なら panic (テスト用)。
#[cfg(test)]
fn refactorize(std: &StdForm, t: &Tableau, prev: Option<&sparse_lu::FtLu>) -> sparse_lu::FtLu {
    try_refactorize(std, t, prev).expect("simplex basis matrix must be nonsingular")
}

/// 現在の基底を LU 分解し直す。数値的に特異なら `None` を返す (呼び出し側はその
/// 経路を諦めて `NotSolved` 等で終える)。`prev` は置き換える前の分解で、あれば
/// そのピボット順を再利用する (検査に通らなければ自動で通常の Markowitz 探索に戻る)。
fn try_refactorize(std: &StdForm, t: &Tableau, prev: Option<&sparse_lu::FtLu>) -> Option<sparse_lu::FtLu> {
    basis_kernel::factorize_basis(std, &t.basis_pos, prev)
}

/// 有界変数主単体法の 1 段階 (第 1 段階または第 2 段階) を実行する。
///
/// - `phase1`: 真なら第 1 段階 (合成目的関数と修正比率テスト)、偽なら第 2 段階 (`std.c`)。
/// - `lu`, `since_check`: Forrest-Tomlin 基底表現と周期検査のカウンタ。両段階で引き継ぐ。
/// - `expand`: EXPAND の作業許容誤差とリセット周期。両段階で引き継ぐ。
/// - `se`: 最急辺重み。`stall`: 停滞検出/Bland モード。
///
/// 戻り値は段階の結果の状態。途中の再分解で基底が数値的に特異になった場合は `None`
/// を返し、このとき `t` の内容は信用できないので呼び出し側は読んではならない
/// (唯一の呼び出し元である `slope_intercept_dual` の仕上げ引き継ぎは `NotSolved` を報告する)。
/// 反復上限に達した場合は最善努力として `Optimal` を返す。
fn run_phase(
    std: &StdForm,
    t: &mut Tableau,
    phase1: bool,
    lu: &mut sparse_lu::FtLu,
    since_check: &mut usize,
    expand: &mut ExpandState,
    se: &mut SteepestEdgeState,
    stall: &mut PrimalStallState,
) -> Option<Status> {
    let m = std.n_rows;
    // 部分価格付けの乱数種 (固定値を n_total で変化させる: 再現性のため)。
    let pricing_seed = 0x2545_F491_4F6C_DD1D_u64 ^ (std.n_total as u64);
    // Bland 規則へ切り替えるまでの停滞ピボット回数の上限。
    let stall_limit = (PRIMAL_STALL_LIMIT_PER_ROW * m).max(PRIMAL_STALL_LIMIT_MIN);

    /// 比率テストでブロックしうる行の候補。
    struct Candidate {
        /// 基底内の行位置。
        row: usize,
        /// 正確な上下限に達するステップ長。
        exact: f64,
        /// `expand.delta` だけ緩めた上下限に達するステップ長。
        relaxed: f64,
        /// ピボット要素の絶対値 `|alpha[row]|`。
        pivot_abs: f64,
        /// 上限側でブロックするか。
        hits_upper: bool,
    }

    // 反復ごとの作業バッファ (ループ外で一度だけ確保する)。
    let mut cost_buf = vec![0.0; m]; // 基底変数の費用 c_B
    let mut y_buf = vec![0.0; m]; // 双対変数 y = B^-T c_B
    let mut a_enter_buf = vec![0.0; m]; // 入る列 A_q (密)
    let mut alpha_buf = vec![0.0; m]; // alpha = B^-1 A_q
    let mut scratch_buf = vec![0.0; m];
    let mut rho_buf = vec![0.0; m]; // B^-1 の第 r 行
    let mut w_buf = vec![0.0; m]; // B^-T alpha
    let mut candidates_buf: Vec<Candidate> = Vec::with_capacity(m);

    // 比率テストでブロック行とみなす |alpha_i| の下限 (これ以下の行は無視)。
    let ratio_pivot_tol = if env_str!("ENOMOTO_PRIMAL_RATIO_PIVOT_TOL_OLD").is_some() { TOL } else { tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64) };
    let max_iters = max_iters_for(m, std.n_total);
    for iter_idx in 0..max_iters {
        if iter_idx & 63 == 0 && crate::cancel::is_cancelled() {
            return None; // 同時実行の相手が先に結論を出した
        }
        prof_phases::RUN_PHASE_ITERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let rhs = t.recompute_basics(lu);

        // 再分解トリガ (1)(3): 周期的な残差検査と eta ファイル fill 検査。
        *since_check += 1;
        if *since_check >= FT_CHECK_INTERVAL {
            *since_check = 0;
            let bump_too_big = lu.fill_count() > tunable!("ENOMOTO_T_FT_BUMP_LIMIT_FACTOR", FT_BUMP_LIMIT_FACTOR, usize) * m.max(1);
            let residual_too_big = !bump_too_big && t.basis_residual_norm(&rhs) > FT_RESIDUAL_TOL;
            if bump_too_big || residual_too_big {
                // 特異なら panic せず None を返す (関数文書参照)。
                let Some(l) = try_refactorize(std, t, Some(&*lu)) else {
                    if env_str!("ENOMOTO_DEBUG_PHASES").is_some() {
                        eprintln!("run_phase None@residual iter={iter_idx} phase1={phase1} bump={bump_too_big} residual={residual_too_big}");
                    }
                    return None;
                };
                *lu = l;
            }
        }
        // 再分解トリガ (4): 更新回数の上限。
        if lu.update_count() > FT_MAX_UPDATES {
            let Some(l) = try_refactorize(std, t, Some(&*lu)) else {
                if env_str!("ENOMOTO_DEBUG_PHASES").is_some() {
                    eprintln!("run_phase None@ft_max_updates iter={iter_idx} phase1={phase1}");
                }
                return None;
            };
            *lu = l;
        }

        // EXPAND (Gill et al. 1989 §4.2): 作業許容誤差を毎反復増やし、EXPAND_K 反復ごとに
        // 非基底値を上下限に戻して新しい拡大系列を始める。
        expand.delta += EXPAND_TAU;
        expand.iters_since_reset += 1;
        if expand.iters_since_reset >= EXPAND_K {
            expand.iters_since_reset = 0;
            expand.delta = EXPAND_DELTA_0;
            t.expand_reset_nonbasics();
        }

        // ---- この反復の基底費用ベクトル ----
        // 第 1 段階では、現在の作業許容誤差 `expand.delta` を超えて上下限を外れた
        // 基底変数に -1/+1 を割り当てる ((7.1)-(7.2))。第 2 段階は c_B。
        if phase1 {
            for i in 0..m {
                let var = t.basis[i];
                let v = t.x[var];
                cost_buf[i] = if v < std.lb[var] - expand.delta {
                    -1.0
                } else if v > std.ub[var] + expand.delta {
                    1.0
                } else {
                    0.0
                };
            }
        } else {
            for i in 0..m {
                cost_buf[i] = std.c[t.basis[i]];
            }
        }
        let cost: &[f64] = &cost_buf;

        if phase1 && cost.iter().all(|&c| c == 0.0) {
            return Some(Status::Optimal); // 第 1 段階: 実行可能になった
        }

        // y = B^-T cost_B、被約費用 d_j = c_j - y・a_j
        lu.solve_transpose_into(cost, &mut scratch_buf, &mut y_buf);
        let y: &[f64] = &y_buf;

        // 入る変数の選択: 最急辺規則 (d_j^2 / gamma_j の最大)。列数が
        // PARTIAL_PRICING_THRESHOLD 以上なら部分価格付け (まず約 1/PARTIAL_PRICING_GROUPS の
        // 標本だけを調べ、改善候補がなければ残りを調べる)。
        // `price_one(j)` は列 j が候補なら `(j, スコア, 移動方向 ±1, d_j)` を返す。
        let price_one = |j: usize| -> Option<(usize, f64, f64, f64)> {
            let st = t.nb_status[j]?;
            // 固定列は入る変数になり得ないので内積の前に除外する。
            if std.lb[j] == std.ub[j] {
                return None;
            }
            let cj = if phase1 { 0.0 } else { std.c[j] };
            let dot = sparse_dot_dense(t.column_sparse(j), y);
            let dj = cj - dot;

            let (eligible, dir) = match st {
                NbStatus::Lower => (dj < -TOL, 1.0),
                NbStatus::Upper => (dj > TOL, -1.0),
                // 値 0 の自由列: 目的を下げる向きに動かす (幅無限なのでフリップはない)。
                NbStatus::Zero => (true, if dj < 0.0 { 1.0 } else { -1.0 }),
            };
            if eligible && dj.abs() > TOL {
                let score = dj * dj / se.gamma[j].max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
                Some((j, score, dir, dj))
            } else {
                None
            }
        };

        // Bland モードでは、全列を走査して最小添字の候補を選ぶ (部分価格付けは使わない)。
        let best = if stall.bland_mode {
            (0..std.n_total).filter_map(price_one).min_by_key(|&(j, ..)| j)
        } else if std.n_total >= tunable!("ENOMOTO_T_PARTIAL_PRICING_THRESHOLD", PARTIAL_PRICING_THRESHOLD, usize) {
            let iter_u64 = iter_idx as u64;
            let sample_best = (0..std.n_total)
                .filter(|&j| partial_pricing_sampled(pricing_seed, iter_u64, j))
                .filter_map(price_one)
                .max_by(|a, b| a.1.total_cmp(&b.1));
            sample_best.or_else(|| {
                (0..std.n_total)
                    .filter(|&j| !partial_pricing_sampled(pricing_seed, iter_u64, j))
                    .filter_map(price_one)
                    .max_by(|a, b| a.1.total_cmp(&b.1))
            })
        } else {
            (0..std.n_total).filter_map(price_one).max_by(|a, b| a.1.total_cmp(&b.1))
        };

        let Some((enter, _best_score, best_dir, dj_enter)) = best else {
            // 改善方向なし。
            return Some(if phase1 { Status::Infeasible } else { Status::Optimal });
        };

        // alpha = B^-1 a_enter
        t.column_into(enter, &mut a_enter_buf);
        lu.solve_into(&a_enter_buf, &mut scratch_buf, &mut alpha_buf);
        let a_enter: &[f64] = &a_enter_buf;
        let alpha: &[f64] = &alpha_buf;

        // ---- 2 パスの Harris/EXPAND 比率テスト (Gill, Murray, Saunders & Wright 1989, §3.2, §4) ----
        // パス 1: `expand.delta` だけ外側に緩めた上下限に最初に達するステップ `alpha1` を求める。
        // パス 2: 正確な上下限へのステップが `alpha1` (+ PRIMAL_HARRIS_TOL) 以内の行のうち
        // ピボットの絶対値が最大の行を選ぶ。最終ステップは `EXPAND_TAU / |pivot|` > 0 で
        // 下から抑える (これが巡回を防ぐ)。出る変数は最大 `expand.delta` だけ上下限を外れうるが、
        // 後で `expand_reset_nonbasics` が戻す。
        let self_width = std.ub[enter] - std.lb[enter]; // 入る変数自身の幅 (フリップまでの距離)
        let init_alpha1 = if self_width.is_finite() { self_width } else { f64::INFINITY };

        // 各行のブロック判定 (逐次実行)。
        candidates_buf.clear();
        for i in 0..m {
            let rate = -best_dir * alpha[i]; // x_Bi のステップあたりの変化率 d(x_Bi)/d(theta)
            // |alpha_i| が小さすぎる行は、出す基底がほぼ特異になるのでブロック行とみなさない
            // (許す上下限違反は高々 theta * ratio_pivot_tol)。
            // `ENOMOTO_PRIMAL_RATIO_PIVOT_TOL_OLD` で旧値 TOL に戻せる (A/B 比較用)。
            if rate.abs() <= ratio_pivot_tol {
                continue;
            }
            let var = t.basis[i];
            let val = t.x[var];
            let infeasible_low = phase1 && val < std.lb[var] - expand.delta; // 第 1 段階で下限違反
            let infeasible_high = phase1 && val > std.ub[var] + expand.delta; // 第 1 段階で上限違反

            // この行をブロックする上下限: 実行可能な行は進行方向の上下限。違反している行
            // (第 1 段階のみ) は実行可能側へ戻る向きの上下限だけでブロックされ、違反を
            // 深める向きはブロックしない (Gill et al. 1989 §7.1)。実行可能へ戻る行は緩め幅なし (relaxed == exact)。
            // `active` = この行がブロックしうるか、`returning_to_feasibility` = 違反から戻る途中か。
            let (bound, is_upper, active, returning_to_feasibility) = if rate < 0.0 {
                if infeasible_high {
                    (std.ub[var], true, true, true)
                } else if infeasible_low {
                    (std.lb[var], false, false, false)
                } else {
                    (std.lb[var], false, true, false)
                }
            } else if infeasible_low {
                (std.lb[var], false, true, true)
            } else if infeasible_high {
                (std.ub[var], true, false, false)
            } else {
                (std.ub[var], true, true, false)
            };

            if !active || !bound.is_finite() {
                continue;
            }
            let exact = (bound - val) / rate;
            let relaxed = if returning_to_feasibility {
                exact
            } else {
                let relaxed_bound = if is_upper { bound + expand.delta } else { bound - expand.delta };
                (relaxed_bound - val) / rate
            };

            candidates_buf.push(Candidate { row: i, exact, relaxed, pivot_abs: alpha[i].abs(), hits_upper: is_upper });
        }
        let candidates: &[Candidate] = &candidates_buf;

        let alpha1 = candidates.iter().map(|c| c.relaxed).fold(init_alpha1, f64::min);

        // パス 2 の受け入れ窓は alpha1 + PRIMAL_HARRIS_TOL。通常はピボット絶対値最大の行、
        // Bland モードでは基底変数の添字が最小の行を出す行とする。
        let admitted = candidates.iter().filter(|c| c.exact <= alpha1 + tunable!("ENOMOTO_T_PRIMAL_HARRIS_TOL", PRIMAL_HARRIS_TOL, f64));
        let leaving = if stall.bland_mode {
            admitted.min_by_key(|c| t.basis[c.row])
        } else {
            admitted.max_by(|a, b| a.pivot_abs.total_cmp(&b.pivot_abs))
        };
        // 出る行 (None ならフリップ)、上限側か、その正確なステップ alpha2、ピボット絶対値。
        let (leaving_row, leaving_hits_upper, alpha2, best_pivot_mag) = match leaving {
            Some(c) if c.pivot_abs > 0.0 => (Some(c.row), c.hits_upper, c.exact, c.pivot_abs),
            _ => (None, false, 0.0, 0.0),
        };

        // 実際のステップ長 (出る行があれば EXPAND の最小ステップで下から抑える)。
        let theta = match leaving_row {
            None => {
                if !alpha1.is_finite() {
                    return Some(Status::Unbounded);
                }
                alpha1
            }
            Some(_) => alpha2.max(EXPAND_TAU / best_pivot_mag),
        };

        // 停滞検出: このピボットの目的への寄与 theta * d_q が小さければ数える。
        if (theta * dj_enter).abs() < STALL_PROGRESS_EPS {
            stall.stall_count += 1;
            if stall.stall_count > stall_limit {
                stall.bland_mode = true;
            }
        } else {
            stall.stall_count = 0;
        }

        // ステップを適用する。
        for i in 0..m {
            let var = t.basis[i];
            t.x[var] -= best_dir * alpha[i] * theta;
        }
        t.x[enter] += best_dir * theta;

        match leaving_row {
            None => {
                // 境界フリップ: 入る変数は反対側の上下限へ移り、非基底のまま。
                let new_status = if best_dir > 0.0 { NbStatus::Upper } else { NbStatus::Lower };
                t.nb_status[enter] = Some(new_status);
                t.x[enter] = if best_dir > 0.0 { std.ub[enter] } else { std.lb[enter] };
            }
            Some(r) => {
                // 最急辺重み更新に必要な rho = B^-1 の第 r 行と w = B^-T alpha を、
                // 基底入れ替え前の LU で計算しておく。
                lu.solve_transpose_unit(r, &mut scratch_buf, &mut rho_buf);
                lu.solve_transpose_into(alpha, &mut scratch_buf, &mut w_buf);
                let rho: &[f64] = &rho_buf;
                let w: &[f64] = &w_buf;
                let gamma_t_old = se.gamma[enter];
                let pivot = alpha[r];

                let leaving_var = t.basis[r];
                t.nb_status[leaving_var] = Some(if leaving_hits_upper { NbStatus::Upper } else { NbStatus::Lower });
                // EXPAND では出る変数を上下限に丸めない (最大 expand.delta の違反を許し、
                // 後で expand_reset_nonbasics が戻す)。
                t.basis_pos[leaving_var] = None;
                t.basis[r] = enter;
                t.basis_pos[enter] = Some(r);
                t.nb_status[enter] = None;

                // 入れ替え後の全非基底列 (出た変数を含み、入った変数を除く) の重みを更新する。
                se.update_after_pivot(t, std, rho, w, gamma_t_old, pivot);

                // 再分解トリガ (2): FT 更新のピボットが小さすぎれば再分解する。
                if !lu.try_update(r, a_enter, tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64)) {
                    let Some(l) = try_refactorize(std, t, Some(&*lu)) else {
                        if env_str!("ENOMOTO_DEBUG_PHASES").is_some() {
                            eprintln!("run_phase None@ft_update iter={iter_idx} phase1={phase1} pivot={pivot}");
                        }
                        return None;
                    };
                    *lu = l;
                }
            }
        }
    }

    Some(Status::Optimal) // 反復上限に到達 (最善努力)
}

/// 差分更新版の第 2 段階主単体法 (`slope_intercept_dual` の仕上げ → 主単体法引き継ぎ専用、
/// 既定で有効、`ENOMOTO_HANDOFF_INCREMENTAL=0` で無効)。
///
/// [`run_phase`] が毎反復すべてを作り直すのに対し、`x_B` と被約費用 `d` を差分更新する:
///
/// - `x_B` は比率テストのステップで動かし、リフレッシュ時点でだけ作り直す。
/// - `d` はピボット行から更新する: `rho = B^-T e_r`, `alpha_r = rho^T A` (rho の非零行に
///   沿った行方向 PRICE) として `d_j -= (d_q / alpha_rq) alpha_rj`、出る列は `-d_q / alpha_rq`。
/// - 最急辺重みは `alpha_r` と `w^T A` (`w = B^-T alpha`) を同じ行方向走査で集めて更新する。
///   既定 (`ENOMOTO_HANDOFF_INC_DEVEX=1`) は主 Devex (参照重み、BTRAN は rho の 1 回だけ)。
///
/// リフレッシュ (`recompute_basics` と残差/fill 検査による再分解、`c_B` の BTRAN による `d` の
/// 作り直し) は、最初の反復、EXPAND リセット時、再分解後、そして「入る候補なし」(Optimal)
/// または「ブロック行なし」(Unbounded) を確定する前に行う (差分値は提案のみ、確定は新しい値で)。
/// 戻り値の意味は [`run_phase`] と同じ。
fn run_phase2_incremental(std: &StdForm, t: &mut Tableau, lu: &mut sparse_lu::FtLu, stall: &mut PrimalStallState) -> Option<Status> {
    use std::sync::atomic::Ordering::Relaxed;
    let m = std.n_rows;
    let n = std.n_total;
    let stall_limit = (PRIMAL_STALL_LIMIT_PER_ROW * m).max(PRIMAL_STALL_LIMIT_MIN);
    let floor = tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64);
    let min_pivot = tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64);
    let harris = tunable!("ENOMOTO_T_PRIMAL_HARRIS_TOL", PRIMAL_HARRIS_TOL, f64);
    // FTRAN・BTRAN・FT 更新・再分解トリガ (双対単体法と共有する部品)。
    let mut kernel = basis_kernel::BasisKernel::new(m, FT_MAX_UPDATES);
    let use_devex = tunable!("ENOMOTO_HANDOFF_INC_DEVEX", 1u8, u8) != 0;
    // cont1 策3 の比率テストを旧版に戻す (A/B 用)。
    let ratio_old = tunable!("ENOMOTO_HANDOFF_RATIO_OLD", 0u8, u8) != 0;
    let mut expand = ExpandState::new();

    let mut gamma: Vec<f64> = if use_devex { vec![1.0; n] } else { SteepestEdgeState::new(std).gamma }; // 最急辺 (または Devex) 重み
    let mut cost_b = vec![0.0; m]; // c_B
    let mut y = vec![0.0; m]; // y = B^-T c_B
    let mut scratch = vec![0.0; m];
    let mut d = vec![0.0; n]; // 差分更新する被約費用
    let mut alpha = vec![0.0; m]; // B^-1 A_q
    let mut rho = vec![0.0; m]; // B^-T e_r
    let mut w = vec![0.0; m]; // B^-T alpha (最急辺のみ)
    let mut pivot_row = vec![0.0; n]; // alpha_r = rho^T A の列成分
    let mut w_row = vec![0.0; n]; // w^T A の列成分
    let mut touched = vec![false; n]; // pivot_row/w_row に書き込んだ列の印
    let mut touched_cols: Vec<usize> = Vec::new(); // 書き込んだ列の一覧 (後でゼロに戻す)

    /// 比率テストでブロックしうる行の候補 ([`run_phase`] と同じ)。
    struct Candidate {
        /// 基底内の行位置。
        row: usize,
        /// 正確な上下限に達するステップ長。
        exact: f64,
        /// `expand.delta` だけ緩めた上下限に達するステップ長。
        relaxed: f64,
        /// ピボット要素の絶対値。
        pivot_abs: f64,
        /// 上限側でブロックするか。
        hits_upper: bool,
    }
    let mut candidates: Vec<Candidate> = Vec::with_capacity(m);
    // 入る列の FTRAN・ピボット行の BTRAN の結果の非ゼロ行 (昇順、`kernel` から)。
    let mut alpha_rows: Vec<usize> = Vec::with_capacity(m);
    let mut rho_rows: Vec<usize> = Vec::with_capacity(m);

    let mut need_fresh = true; // 次の反復の頭で x_B と d を作り直すか
    let max_iters = max_iters_for(m, n);
    // 計測用 (`ENOMOTO_DEBUG_PROGRESS=1`): 1000 反復ごとに経過時間・目的値・双対実行不能の列数を出す。
    let progress = env_str!("ENOMOTO_DEBUG_PROGRESS").is_some();
    let progress_t0 = std::time::Instant::now();
    for _iter in 0..max_iters {
        if _iter & 63 == 0 && crate::cancel::is_cancelled() {
            return None; // 同時実行の相手が先に結論を出した
        }
        if progress && _iter % 1000 == 0 {
            let obj: f64 = (0..n).map(|j| std.c[j] * t.x[j]).sum();
            let ndi = (0..n)
                .filter(|&j| match t.nb_status[j] {
                    Some(NbStatus::Lower) => d[j] < -TOL && std.lb[j] < std.ub[j],
                    Some(NbStatus::Upper) => d[j] > TOL && std.lb[j] < std.ub[j],
                    Some(NbStatus::Zero) => d[j].abs() > TOL,
                    None => false,
                })
                .count();
            eprintln!("PROGRESS primal iter={_iter} t={:.1}s obj={obj:.10e} dual_infeas={ndi} bland={}", progress_t0.elapsed().as_secs_f64(), stall.bland_mode);
        }
        prof_phases::RUN_PHASE_ITERS.fetch_add(1, Relaxed);
        let fresh_now = need_fresh; // この反復の x_B/d が作り直したばかりの値か
        if need_fresh {
            need_fresh = false;
            let rhs = t.recompute_basics(lu);
            if kernel.fill_too_big(lu) || t.basis_residual_norm(&rhs) > FT_RESIDUAL_TOL {
                *lu = try_refactorize(std, t, Some(&*lu))?;
                t.recompute_basics(lu);
            }
            for i in 0..m {
                cost_b[i] = std.c[t.basis[i]];
            }
            lu.solve_transpose_into(&cost_b, &mut scratch, &mut y);
            for j in 0..n {
                d[j] = if t.nb_status[j].is_some() { std.c[j] - sparse_dot_dense(t.column_sparse(j), &y) } else { 0.0 };
            }
        }

        expand.delta += EXPAND_TAU;
        expand.iters_since_reset += 1;
        if expand.iters_since_reset >= EXPAND_K {
            expand.iters_since_reset = 0;
            expand.delta = EXPAND_DELTA_0;
            t.expand_reset_nonbasics();
            need_fresh = true;
            continue;
        }

        // 差分更新した `d` で価格付けする (適格条件は `run_phase` の `price_one` と同じ)。
        let mut best: Option<(usize, f64, f64, f64)> = None;
        for j in 0..n {
            let Some(st) = t.nb_status[j] else { continue };
            if std.lb[j] == std.ub[j] {
                continue;
            }
            let dj = d[j];
            let (eligible, dir) = match st {
                NbStatus::Lower => (dj < -TOL, 1.0),
                NbStatus::Upper => (dj > TOL, -1.0),
                NbStatus::Zero => (true, if dj < 0.0 { 1.0 } else { -1.0 }),
            };
            if !(eligible && dj.abs() > TOL) {
                continue;
            }
            if stall.bland_mode {
                if best.is_none() {
                    best = Some((j, 0.0, dir, dj));
                }
                continue;
            }
            let score = dj * dj / gamma[j].max(floor);
            if best.map_or(true, |b| score > b.1) {
                best = Some((j, score, dir, dj));
            }
        }
        let Some((enter, _, best_dir, dj_enter)) = best else {
            if fresh_now {
                return Some(Status::Optimal);
            }
            need_fresh = true;
            continue;
        };

        kernel.ftran_col(lu, t.column_sparse(enter), &mut alpha);
        // 結果の非ゼロ行 (昇順。大きな問題では FTRAN の記録から、長さ `m` の走査をしない)。
        kernel.ftran_rows_into(&alpha, &mut alpha_rows);

        // 比率テスト: EXPAND 2 パス (Gill et al. 1989)。
        //
        // cont1 策3: 旧版 (`ENOMOTO_HANDOFF_RATIO_OLD=1`) は `run_phase` の第 2 段階と同じく
        // パス 2 の窓を `exact <= alpha1 + PRIMAL_HARRIS_TOL` (ステップ長の絶対値) にしていたので、
        // 窓の中の他の行の行き過ぎが `harris * |alpha_i|` になり、`B^-1` が密で `|alpha|` が 100 級の
        // cont1 では 1e-5〜1e-3 の上下限違反を作って非基底化していた。EXPAND の緩め幅 `delta` が
        // すでに変数空間の許容幅なので、パス 2 は `exact <= alpha1` (追加の窓なし) にする
        // (他の行の行き過ぎは `delta` に収まる)。出る変数自身の行き過ぎは下の「出る変数を境界へ」で扱う。
        // 報告の案にあった「すでに外れた基底変数は戻る向きだけブロックする」(`run_phase` の第 1 段階の
        // 規則) も試したが、cont1 の引き継ぎが 2,213 → 4,123 反復に増えた (違反を深める向きで
        // ブロックしないので外れた変数がさらに外れ、最後の双対ループの仕事も増える) ので採らない。
        let self_width = std.ub[enter] - std.lb[enter];
        let init_alpha1 = if self_width.is_finite() { self_width } else { f64::INFINITY };
        candidates.clear();
        for &i in &alpha_rows {
            let rate = -best_dir * alpha[i];
            if rate.abs() <= min_pivot {
                continue;
            }
            let var = t.basis[i];
            let val = t.x[var];
            let (bound, is_upper) = if rate < 0.0 { (std.lb[var], false) } else { (std.ub[var], true) };
            if !bound.is_finite() {
                continue;
            }
            let exact = (bound - val) / rate;
            let relaxed_bound = if is_upper { bound + expand.delta } else { bound - expand.delta };
            let relaxed = (relaxed_bound - val) / rate;
            candidates.push(Candidate { row: i, exact, relaxed, pivot_abs: alpha[i].abs(), hits_upper: is_upper });
        }
        let alpha1 = candidates.iter().map(|c| c.relaxed).fold(init_alpha1, f64::min);
        let window = if ratio_old { harris } else { 0.0 };
        let admitted = candidates.iter().filter(|c| c.exact <= alpha1 + window);
        let leaving = if stall.bland_mode { admitted.min_by_key(|c| t.basis[c.row]) } else { admitted.max_by(|a, b| a.pivot_abs.total_cmp(&b.pivot_abs)) };
        let (leaving_row, leaving_hits_upper, alpha2, best_pivot_mag) = match leaving {
            Some(c) if c.pivot_abs > 0.0 => (Some(c.row), c.hits_upper, c.exact, c.pivot_abs),
            _ => (None, false, 0.0, 0.0),
        };
        let theta = match leaving_row {
            None => {
                if !alpha1.is_finite() {
                    if fresh_now {
                        return Some(Status::Unbounded);
                    }
                    need_fresh = true;
                    continue;
                }
                alpha1
            }
            Some(_) => alpha2.max(EXPAND_TAU / best_pivot_mag),
        };

        if (theta * dj_enter).abs() < STALL_PROGRESS_EPS {
            stall.stall_count += 1;
            if stall.stall_count > stall_limit {
                stall.bland_mode = true;
            }
        } else {
            stall.stall_count = 0;
        }

        for &i in &alpha_rows {
            let var = t.basis[i];
            t.x[var] -= best_dir * alpha[i] * theta;
        }
        t.x[enter] += best_dir * theta;

        let Some(r) = leaving_row else {
            let new_status = if best_dir > 0.0 { NbStatus::Upper } else { NbStatus::Lower };
            t.nb_status[enter] = Some(new_status);
            t.x[enter] = if best_dir > 0.0 { std.ub[enter] } else { std.lb[enter] };
            continue;
        };

        // 入れ替え前の基底でピボット行 (と最急辺なら `w^T A`) を行方向に集める。
        kernel.btran_row(lu, r, &mut rho);
        if use_devex {
            kernel.btran_rows_into(&rho, &mut rho_rows);
        } else {
            // 最急辺の `w = B^-T alpha` は密なので全行を見る。
            lu.solve_transpose_into(&alpha, &mut scratch, &mut w);
            rho_rows.clear();
            rho_rows.extend(0..m);
        }
        for &i in &rho_rows {
            let rv = rho[i];
            let wv = if use_devex { 0.0 } else { w[i] };
            if rv.abs() <= TOL && wv.abs() <= TOL {
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
                pivot_row[j] += rv * v;
                w_row[j] += wv * v;
            }
        }

        let pivot = alpha[r];
        let theta_d = d[enter] / pivot; // 双対ステップ d_q / alpha_rq
        let gamma_q = gamma[enter];

        let leaving_var = t.basis[r];
        t.nb_status[leaving_var] = Some(if leaving_hits_upper { NbStatus::Upper } else { NbStatus::Lower });
        t.basis_pos[leaving_var] = None;
        t.basis[r] = enter;
        t.basis_pos[enter] = Some(r);
        t.nb_status[enter] = None;
        // cont1 策3: 出る変数が EXPAND の許容幅 `delta` を超えて上下限を外れたまま非基底になる
        // (すでに外れていた基底変数が出る場合や、最小ステップ `EXPAND_TAU / |pivot|` がピボットの
        // 小さい行で大きくなる場合) なら、境界に置き直して次の反復の頭で `x_B` を作り直す
        // (HiGHS の主単体法と同じく出る変数は境界に置く。`delta` 以内の外れは従来どおり
        // `expand_reset_nonbasics` に任せるので、通常の反復は変わらない)。旧版はこの値のまま
        // 非基底に残し、`expand_reset_nonbasics` も 1e-6 を超える外れは戻さないので、外れが残り続けた。
        if !ratio_old {
            let bound = if leaving_hits_upper { std.ub[leaving_var] } else { std.lb[leaving_var] };
            if (t.x[leaving_var] - bound).abs() > expand.delta {
                t.x[leaving_var] = bound;
                need_fresh = true;
            }
        }

        for &j in &touched_cols {
            if t.nb_status[j].is_some() {
                let beta = pivot_row[j] / pivot;
                if j != leaving_var {
                    d[j] -= theta_d * pivot_row[j];
                    if use_devex {
                        gamma[j] = gamma[j].max(beta * beta * gamma_q);
                    }
                }
                if !use_devex {
                    gamma[j] = (gamma[j] + beta * beta * (1.0 + gamma_q) - 2.0 * beta * w_row[j]).max(floor);
                }
            }
            pivot_row[j] = 0.0;
            w_row[j] = 0.0;
            touched[j] = false;
        }
        touched_cols.clear();
        d[leaving_var] = -theta_d;
        d[enter] = 0.0;
        if use_devex {
            gamma[leaving_var] = (gamma_q / (pivot * pivot)).max(1.0);
        }

        // FT 更新 (FTRAN/BTRAN の途中値を使う) と再分解トリガ (3)(4)(5)。残差検査 (1) はリフレッシュ時点でのみ行う。
        if kernel.update_and_check(lu, r).is_due() {
            *lu = try_refactorize(std, t, Some(&*lu))?;
            need_fresh = true;
        }
    }

    Some(Status::Optimal) // 反復上限に到達 (最善努力、`run_phase` と同じ)
}

/// `std` の構造変数を連結成分に分ける (同じ行に構造変数として現れる 2 変数を連結とみなす)。
///
/// スラック列は各行固有なので連結の判定には使わない。前処理は変数間の結合を
/// 増やさないので、前処理後の `std` で 1 回調べれば十分。
/// 成分が 1 つしかない場合、または構造変数を 1 つも含まない行 (実行不能の証拠として
/// 残された行の可能性がある) がある場合は `None`。
/// 成分と一緒に `has_row[j]` (変数 `j` が少なくとも 1 行に現れるか) も返し、
/// [`solve_std_form_decomposed`] が分割の価値を安く判定するのに使う。
fn connected_components_of_std_form(std: &StdForm) -> Option<(Vec<Vec<usize>>, Vec<bool>)> {
    let n_orig = std.n_total - std.n_rows;
    let mut parent: Vec<usize> = (0..n_orig).collect(); // union-find の親
    let mut has_row = vec![false; n_orig];
    /// union-find の根を経路圧縮つきで求める。
    fn find(parent: &mut [usize], x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }
    /// `a` と `b` の成分を併合する。
    fn union(parent: &mut [usize], a: usize, b: usize) {
        let (ra, rb) = (find(parent, a), find(parent, b));
        if ra != rb {
            parent[ra] = rb;
        }
    }

    for i in 0..std.n_rows {
        let mut first: Option<usize> = None; // この行で最初に見た構造変数
        for &(j, _) in std.rows.row(i) {
            if j >= n_orig {
                continue; // この行のスラック列
            }
            has_row[j] = true;
            match first {
                None => first = Some(j),
                Some(f) => union(&mut parent, f, j),
            }
        }
        // 構造変数を含まない行 (スラックのみ) は、doubleton が残した実行不能の
        // 証拠行かもしれないので、分割せず従来の一括求解に任せる。
        if first.is_none() {
            return None;
        }
    }

    let mut groups: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new(); // 根 → 成分の変数一覧
    for j in 0..n_orig {
        let root = find(&mut parent, j);
        groups.entry(root).or_default().push(j);
    }
    if groups.len() <= 1 {
        return None;
    }
    Some((groups.into_values().collect(), has_row))
}

/// [`connected_components_of_std_form`] の各成分ごとに独立した [`StdForm`] を作る
/// (全成分をまとめて `O(nnz)` の 1 パスで構築)。
///
/// 各行はその構造変数が属する成分に割り当てる (1 行が 2 成分にまたがることはない)。
/// 変数は成分内の順に `0..component.len()` へ付け直し、各行には元のスラックの
/// 上下限を持つ新しいローカルなスラック列を付ける。
fn split_std_form(std: &StdForm, components: &[Vec<usize>]) -> Vec<StdForm> {
    let n_orig = std.n_total - std.n_rows;
    let mut comp_id = vec![usize::MAX; n_orig]; // 変数 → 成分番号
    let mut local_idx = vec![usize::MAX; n_orig]; // 変数 → 成分内の番号
    for (cid, comp) in components.iter().enumerate() {
        for (local_j, &orig_j) in comp.iter().enumerate() {
            comp_id[orig_j] = cid;
            local_idx[orig_j] = local_j;
        }
    }

    // 成分ごとに蓄積する行・右辺・上下限・費用。
    let mut rows_acc: Vec<Vec<Vec<(usize, f64)>>> = vec![Vec::new(); components.len()];
    let mut b_acc: Vec<Vec<f64>> = vec![Vec::new(); components.len()];
    let mut lb_acc: Vec<Vec<f64>> = components.iter().map(|c| c.iter().map(|&j| std.lb[j]).collect()).collect();
    let mut ub_acc: Vec<Vec<f64>> = components.iter().map(|c| c.iter().map(|&j| std.ub[j]).collect()).collect();
    let mut c_acc: Vec<Vec<f64>> = components.iter().map(|c| c.iter().map(|&j| std.c[j]).collect()).collect();

    for i in 0..std.n_rows {
        let cid = std
            .rows
            .row(i)
            .iter()
            .find_map(|&(j, _)| if j < n_orig { Some(comp_id[j]) } else { None })
            .expect("row with no structural members must have made connected_components_of_std_form bail out already");
        let local_n = components[cid].len();
        let slack_col = local_n + rows_acc[cid].len(); // この行の成分内でのスラック列番号
        let mut row: Vec<(usize, f64)> = Vec::with_capacity(std.rows.row(i).len());
        let (mut slack_lb, mut slack_ub) = (0.0, 0.0);
        for &(j, v) in std.rows.row(i) {
            if j < n_orig {
                debug_assert_eq!(comp_id[j], cid, "row split across two components");
                row.push((local_idx[j], v));
            } else {
                row.push((slack_col, v));
                slack_lb = std.lb[j];
                slack_ub = std.ub[j];
            }
        }
        rows_acc[cid].push(row);
        b_acc[cid].push(std.b[i]);
        lb_acc[cid].push(slack_lb);
        ub_acc[cid].push(slack_ub);
        c_acc[cid].push(0.0);
    }

    let mut result = Vec::with_capacity(components.len());
    for cid in 0..components.len() {
        let local_n = components[cid].len();
        let rows = std::mem::take(&mut rows_acc[cid]);
        let n_rows = rows.len();
        let n_total = local_n + n_rows;
        let (rows, cols) = freeze_std_matrices(&rows, n_total);
        result.push(StdForm {
            n_total,
            n_rows,
            c: std::mem::take(&mut c_acc[cid]),
            rows,
            cols,
            b: std::mem::take(&mut b_acc[cid]),
            lb: std::mem::take(&mut lb_acc[cid]),
            ub: std::mem::take(&mut ub_acc[cid]),
        });
    }
    result
}

/// 前処理済みの `StdForm` を [`slope_intercept_dual::solve_slope_intercept_dual`] で解く。
///
/// [`connected_components_of_std_form`] が「実質的な」成分 (2 変数以上、または行に
/// 現れる 1 変数) を 2 つ以上見つけたときだけ成分ごとに分けて解き、元の変数番号で
/// 1 つの [`SimplexResult`] に組み立てる (状態の合成は [`combine_component_statuses`])。
/// どれかの成分が [`PARALLEL_COMPONENT_MIN_VARS`] 以上なら rayon で並列に解く。
/// 行を持たない孤立変数だけを切り出しても利益がない (かえって遅く、ピボット経路も
/// 変わる) ので、その場合は分割せず一括で解く。傾き・切片二段解法が `None` を返したら
/// [`Status::NotSolved`] とする。
fn solve_std_form_decomposed(std: &StdForm, opts: &crate::types::LpOptions) -> SimplexResult {
    // 1 つの標準形を傾き・切片二段解法で解く (諦めたら NotSolved)。
    let solve_one = |s: &StdForm| {
        dualize::solve(s, opts)
            .or_else(|| sifting::solve(s, opts))
            .or_else(|| slope_intercept_dual::solve_slope_intercept_dual(s, opts))
            .unwrap_or(SimplexResult { status: Status::NotSolved, x: None })
    };

    let Some((components, has_row)) = connected_components_of_std_form(std) else {
        return solve_one(std);
    };
    // 実質的な成分の数 (2 変数以上か、行に現れる 1 変数)。
    let real_components = components.iter().filter(|c| c.len() > 1 || has_row[c[0]]).count();
    if real_components <= 1 {
        return solve_one(std);
    }

    solve_split(std, &components, solve_one)
}

/// 変数の組 `groups` (互いに行を共有しない) ごとに `std` を分けて `solve_one` で解き、元の変数番号で
/// 1 つの [`SimplexResult`] に組み立てる。どれかの組が [`PARALLEL_COMPONENT_MIN_VARS`] 以上なら rayon で並列に解く。
fn solve_split(std: &StdForm, groups: &[Vec<usize>], solve_one: impl Fn(&StdForm) -> SimplexResult + Sync) -> SimplexResult {
    let sub_std_forms = split_std_form(std, groups);
    let use_parallel = groups.iter().any(|c| c.len() >= PARALLEL_COMPONENT_MIN_VARS); // rayon で並列に解くか
    let results: Vec<SimplexResult> = if use_parallel {
        use rayon::prelude::*;
        // 打ち切りのトークン (同時実行時) を各成分のタスクに引き継ぐ (`crate::cancel`)。
        let token = crate::cancel::current();
        // 試験用 `ENOMOTO_T_SPLIT_INNER_SEQ=1`: 大きな組の数がスレッド数以上なら、外側の並列でスレッドが埋まるので、
        // 組の中の分解 (内点法の正規方程式) は並列にしない (入れ子の並列の分割・同期の手間と、待ち合わせ中に別の組の
        // 仕事を盗んで組の進み方がばらつくのを避ける)。
        let n_big = groups.iter().filter(|c| c.len() >= PARALLEL_COMPONENT_MIN_VARS).count();
        let inner_seq = tunable!("ENOMOTO_T_SPLIT_INNER_SEQ", 0u8, u8) != 0 && n_big >= rayon::current_num_threads();
        sub_std_forms
            .par_iter()
            .map(|s| crate::cancel::with_token(token.clone(), || crate::interior_point::kkt::with_inner_seq(inner_seq, || solve_one(s))))
            .collect()
    } else {
        sub_std_forms.iter().map(&solve_one).collect()
    };

    let status = combine_component_statuses(results.iter().map(|r| &r.status));
    if status != Status::Optimal {
        return SimplexResult { status, x: None };
    }
    let n_orig = std.n_total - std.n_rows;
    let mut x = vec![0.0; n_orig];
    for (result, component) in results.iter().zip(groups.iter()) {
        let sub_x = result.x.as_ref().expect("Optimal result must carry x");
        for (local_j, &orig_j) in component.iter().enumerate() {
            x[orig_j] = sub_x[local_j];
        }
    }
    SimplexResult { status: Status::Optimal, x: Some(x) }
}

/// 内点法 + クロスオーバーを、独立な成分ごとに分けて行う (fome13 = dfl001 の 8 個の直和など)。
/// 変数が `ENOMOTO_T_XO_SPLIT_MIN_VARS` (既定 [`XO_SPLIT_MIN_VARS`]) 以上の成分は 1 つずつ、それより小さい成分は
/// まとめて 1 つの問題として解く。分けられる組が 2 つ未満なら `None` (呼び出し側が一括で解く)。
/// 各組で内点法が収束しない・基底が作れないときは、その組だけ傾き・切片二段解法で解き直す。
fn solve_ipm_crossover_split(std: &StdForm, opts: &crate::types::LpOptions) -> Option<SimplexResult> {
    let groups = ipm_crossover_split_groups(std)?;
    Some(solve_split(std, &groups, |s| {
        crossover::solve_ipm_crossover(s).unwrap_or_else(|| {
            crate::phase_timing::mark("crossover_fallback");
            solve_std_form_decomposed(s, opts)
        })
    }))
}

/// 同時実行 ([`race`]) の内点法 + クロスオーバー側: [`solve_ipm_crossover_split`] と同じく独立な成分ごとに分けて解く
/// (fome13 は 8 成分をまとめて 1 つの内点法で解くと終盤の精度が出ず、頂点が採用されないことがある)。
/// どれかの組で内点法が収束しなければ、その組を二段解法で解き直さずに `None` を返す (二段解法は同時に走っている)。
/// 分けられなければ問題全体を解く。
pub(super) fn solve_ipm_crossover_race(std: &StdForm) -> Option<SimplexResult> {
    if tunable!("ENOMOTO_T_RACE_XO_SPLIT", 1u8, u8) == 0 {
        return crossover::solve_ipm_crossover(std);
    }
    let Some(groups) = ipm_crossover_split_groups(std) else {
        return crossover::solve_ipm_crossover(std);
    };
    let r = solve_split(std, &groups, |s| crossover::solve_ipm_crossover(s).unwrap_or(SimplexResult { status: Status::NotSolved, x: None }));
    (r.status != Status::NotSolved).then_some(r)
}

/// [`solve_ipm_crossover_split`] の組: 変数が `ENOMOTO_T_XO_SPLIT_MIN_VARS` (既定 [`XO_SPLIT_MIN_VARS`]) 以上の成分は 1 つずつ、
/// それより小さい成分はまとめて 1 つ。分けられる組が 2 つ未満なら `None`。
fn ipm_crossover_split_groups(std: &StdForm) -> Option<Vec<Vec<usize>>> {
    let min_vars = tunable!("ENOMOTO_T_XO_SPLIT_MIN_VARS", XO_SPLIT_MIN_VARS, usize);
    if min_vars == 0 {
        return None;
    }
    let (components, _) = connected_components_of_std_form(std)?;
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut rest: Vec<usize> = Vec::new();
    for c in components {
        if c.len() >= min_vars {
            groups.push(c);
        } else {
            rest.extend(c);
        }
    }
    if groups.is_empty() || (groups.len() == 1 && rest.is_empty()) {
        return None;
    }
    if !rest.is_empty() {
        rest.sort_unstable();
        groups.push(rest);
    }
    crate::phase_timing::record("xo_split_groups", groups.len() as f64);
    Some(groups)
}

/// 独立な成分それぞれの状態から問題全体の状態を決める。
///
/// 1 つでも `Infeasible` なら全体も `Infeasible`。そうでなく有限最適解のない成分
/// (`InfeasibleOrUnbounded`/`Unbounded`) があれば全体も有限最適解なし。ただし全体を
/// `Unbounded` とするには他の成分が実行可能である必要があるので、`NotSolved` と
/// `Unbounded` が混在すれば `InfeasibleOrUnbounded`。全成分が `Optimal` なら `Optimal`。
fn combine_component_statuses<'a>(statuses: impl Iterator<Item = &'a Status>) -> Status {
    let (mut infeasible, mut infeasible_or_unbounded, mut unbounded, mut not_solved) = (false, false, false, false);
    for s in statuses {
        match s {
            Status::Infeasible => infeasible = true,
            Status::InfeasibleOrUnbounded => infeasible_or_unbounded = true,
            Status::Unbounded => unbounded = true,
            Status::NotSolved | Status::TimeLimit | Status::NodeLimit => not_solved = true,
            Status::Optimal => {}
        }
    }
    if infeasible {
        Status::Infeasible
    } else if infeasible_or_unbounded || (unbounded && not_solved) {
        Status::InfeasibleOrUnbounded
    } else if unbounded {
        Status::Unbounded
    } else if not_solved {
        Status::NotSolved
    } else {
        Status::Optimal
    }
}

/// 双対最急辺 (DSE) 重み (Forrest & Goldfarb 1992、式は Huangfu & Hall
/// arXiv:1503.01889 §2.2.1/2.2.3)。
///
/// 基底の行 `i` について `w[i] = ||e_i^T B^-1||^2`。`chuzr` (出る行の選択) は
/// 主実行不能な行のうち `delta_i^2 / w[i]` が最大の行を選ぶ。
struct DseState {
    /// 行ごとの DSE 重み (長さ m)。
    w: Vec<f64>,
    /// 重み更新を rayon で並列化するか (構築時に `m > RAYON_SIZE_THRESHOLD` で決める)。
    use_parallel: bool,
}

impl DseState {
    /// 初期基底 (符号付き単位行列) に対する重み `w[i] = 1` で作る。
    fn new(m: usize) -> Self {
        DseState { w: vec![1.0; m], use_parallel: m > RAYON_SIZE_THRESHOLD }
    }

    /// 与えた重みで作る (前回の求解の最後の重みを引き継ぐとき)。
    fn from_weights(w: Vec<f64>) -> Self {
        let m = w.len();
        DseState { w, use_parallel: m > RAYON_SIZE_THRESHOLD }
    }

    /// 基底の行 `i` の現在の重み。
    #[inline]
    fn weight(&self, i: usize) -> f64 {
        self.w[i]
    }

    /// 任意の (分解済みの) 基底に対する正確な重みを計算する。行 `i` ごとに BTRAN
    /// (`B^T z = e_i`) を 1 回行い `w[i] = ||z||^2` とする (計 m 回)。`lu` は重みを
    /// 求めたい基底の分解でなければならない (`slope_intercept_dual` は再分解の直後に呼ぶ)。
    fn from_basis(m: usize, lu: &sparse_lu::FtLu) -> Self {
        let mut w = vec![1.0; m];
        let mut scratch = vec![0.0; m];
        let mut z = vec![0.0; m];
        // 疎な単位ベクトル BTRAN は更新なしの分解でのみ正確なので、更新済みなら
        // 通常の密な BTRAN を使う。
        if lu.update_count() == 0 {
            for i in 0..m {
                lu.solve_transpose_unit_into(i, &mut scratch, &mut z);
                let norm_sq: f64 = z.iter().map(|&v| v * v).sum();
                w[i] = norm_sq.max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
            }
        } else {
            for i in 0..m {
                lu.solve_transpose_unit(i, &mut scratch, &mut z);
                let norm_sq: f64 = z.iter().map(|&v| v * v).sum();
                w[i] = norm_sq.max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
            }
        }
        DseState { w, use_parallel: m > RAYON_SIZE_THRESHOLD }
    }

    /// ピボット後の DSE 重み更新。
    ///
    /// - `p`: ピボット行 (変数が出た基底位置)
    /// - `alpha`: 入る列の FTRAN `B^-1 a_q` (ピボット前の基底)
    /// - `tau`: `B^-1 (B^-T e_p)` (DSE 用の追加 FTRAN)
    /// - `rho_p`: `B^-T e_p` (同じ反復の PRICE 用 BTRAN、ピボット前)
    ///
    /// ピボット行の旧重み `wp_old` は、保持している `self.w[p]` ではなく `||rho_p||^2` から
    /// 計算し直す (保持値のドリフトが全行に伝播するのを防ぐ)。
    fn update_after_pivot(&mut self, p: usize, alpha: &[f64], tau: &[f64], rho_p: &[f64]) {
        let pivot = alpha[p];
        // 作業 #10 (J): 下限は関数の入口で 1 回だけ読む (ループ内の `tunable!` は `OnceLock` の atomic 読み出しで、
        // 毎要素の分岐とベクトル化の妨げになっていた)。値は同じ。
        let floor = tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64);
        let wp_old = rho_p.iter().map(|v| v * v).sum::<f64>().max(floor);
        let update_one = |i: usize, w_i: &mut f64| {
            if i == p {
                return;
            }
            let ratio = alpha[i] / pivot;
            *w_i = (*w_i - 2.0 * ratio * tau[i] + ratio * ratio * wp_old).max(floor);
        };
        if self.use_parallel {
            use rayon::prelude::*;
            self.w.par_iter_mut().enumerate().for_each(|(i, w_i)| update_one(i, w_i));
        } else {
            // 行 p も含めて分岐なしで全行を更新する (w[p] は直後に上書きするので
            // 結果は同じで、SIMD 化しやすい)。
            let m = self.w.len();
            for ((w_i, &a_i), &t_i) in self.w.iter_mut().zip(&alpha[..m]).zip(&tau[..m]) {
                let ratio = a_i / pivot;
                *w_i = (*w_i - 2.0 * ratio * t_i + ratio * ratio * wp_old).max(floor);
            }
        }
        self.w[p] = (wp_old / (pivot * pivot)).max(floor);
    }

    /// [`Self::update_after_pivot`] を `rows` (alpha の非零行を含む任意順の行集合) に
    /// 限定したもの。`alpha[i] == 0` の行は更新しても値が変わらないので結果は同一で、
    /// 手間が `O(m)` から `O(nnz(alpha))` に減る。`wp_old` は呼び出し側で計算した
    /// `Σ rho_p[i]^2` (昇順・零を飛ばした和) を渡す。
    fn update_after_pivot_rows(&mut self, p: usize, alpha: &[f64], tau: &[f64], wp_old: f64, rows: &[u32]) {
        let floor = tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64);
        let pivot = alpha[p];
        let wp_old = wp_old.max(floor);
        for &i in rows {
            let i = i as usize;
            let ratio = alpha[i] / pivot;
            self.w[i] = (self.w[i] - 2.0 * ratio * tau[i] + ratio * ratio * wp_old).max(floor);
        }
        self.w[p] = (wp_old / (pivot * pivot)).max(floor);
    }
}

/// 現在主実行不能な基底行の集合 (超疎な `chuzr` 用)。毎反復全行を走査する代わりに
/// ピボットごとに差分更新する。
///
/// 実行可能な行は `chuzr` のスコアがちょうど 0 で選ばれ得ないので、候補はこの集合だけで十分。
/// 行の実行可能性が変わるのは `x_B` の値か基底変数が変わるときだけで、それらは既存の
/// O(m) ループ内で行ごとに `set` を呼んで追跡する (HiGHS の `HEkkDualRHS::workIndex` 相当)。
/// `slope_intercept_dual` からも使われる。
pub(super) struct InfeasibleRows {
    /// 現在実行不能な行番号 (順不同)。
    pub(super) rows: Vec<usize>,
    /// `pos[i] == k` ⇔ `rows[k] == i`、集合外なら [`INFEASIBLE_ROWS_NONE`] (O(1) の所属判定と削除用)。
    /// `Option<usize>` (16 バイト) ではなく `u32` にして、`x_B` 更新で全行を走査する問題
    /// (ex10: 1 反復 5 万行) のメモリ量を減らす。
    pos: Vec<u32>,
}

/// [`InfeasibleRows::pos`] の「集合外」の印。
const INFEASIBLE_ROWS_NONE: u32 = u32::MAX;

impl InfeasibleRows {
    /// m 行分の空の集合を作る。
    pub(super) fn new(m: usize) -> Self {
        assert!(m < INFEASIBLE_ROWS_NONE as usize, "InfeasibleRows: too many rows for u32 positions");
        InfeasibleRows { rows: Vec::new(), pos: vec![INFEASIBLE_ROWS_NONE; m] }
    }

    /// 行 `i` の所属を `infeasible` に設定する (既にその状態なら何もしない)。O(1)。
    pub(super) fn set(&mut self, i: usize, infeasible: bool) {
        let p = self.pos[i];
        if infeasible {
            if p == INFEASIBLE_ROWS_NONE {
                self.pos[i] = self.rows.len() as u32;
                self.rows.push(i);
            }
        } else if p != INFEASIBLE_ROWS_NONE {
            let idx = p as usize;
            let last = self.rows.len() - 1;
            self.rows.swap(idx, last);
            self.rows.pop();
            if idx < self.rows.len() {
                self.pos[self.rows[idx]] = idx as u32;
            }
            self.pos[i] = INFEASIBLE_ROWS_NONE;
        }
    }

    /// 行 `i` が集合に含まれるか。
    #[inline]
    pub(super) fn contains(&self, i: usize) -> bool {
        self.pos[i] != INFEASIBLE_ROWS_NONE
    }

    /// 述語 `pred(i)` (行 i が実行不能か) で集合を O(m) で作り直す。多数の行の `x_B` を
    /// 一度に変える操作 (`resync_basics` や初期化) の直後に使う。
    pub(super) fn rebuild(&mut self, m: usize, mut pred: impl FnMut(usize) -> bool) {
        self.rows.clear();
        for i in 0..m {
            if pred(i) {
                self.pos[i] = self.rows.len() as u32;
                self.rows.push(i);
            } else {
                self.pos[i] = INFEASIBLE_ROWS_NONE;
            }
        }
    }
}

/// `slope_intercept_dual` の診断出力が読むプロセス全体のカウンタ。
mod prof_phases {
    use std::sync::atomic::AtomicUsize;
    /// 主単体法ループ ([`super::run_phase`] 等) の累計反復数 (プロセス全体)。
    /// `ENOMOTO_DEBUG_EXT_ITERS` の診断が前後差で読む。
    pub(crate) static RUN_PHASE_ITERS: AtomicUsize = AtomicUsize::new(0);
}

/// 既定の [`crate::types::LpOptions`] で LP を解く (テスト用)。
#[cfg(test)]
pub fn solve_lp_dual(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow]) -> SimplexResult {
    solve_lp_dual_with(variables, objective, constraints, crate::types::LpOptions::default())
}

/// モデルの LP を前処理 → 傾き・切片二段解法 → 後処理で解く (オプション指定版)。
///
/// 既定では状態は `Optimal`/`Infeasible`/`InfeasibleOrUnbounded` のいずれか
/// (論文の系 7.3 (i))。他の経路で得た `Unbounded` も
/// `opts.distinguish_infeasible_unbounded` が偽なら `InfeasibleOrUnbounded` にまとめる。
/// 傾き・切片二段解法が解を出せなければ [`Status::NotSolved`]。
pub fn solve_lp_dual_with(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow], opts: crate::types::LpOptions) -> SimplexResult {
    let result = solve_lp_dual_full_status(variables, objective, constraints, opts);
    if result.status == Status::Unbounded && !opts.distinguish_infeasible_unbounded {
        return SimplexResult { status: Status::InfeasibleOrUnbounded, x: None };
    }
    result
}

/// [`solve_lp_dual_with`] の本体: `Unbounded` をまとめる前の状態をそのまま返す。
fn solve_lp_dual_full_status(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow], opts: crate::types::LpOptions) -> SimplexResult {
    crate::phase_timing::start();
    let PresolvedForm { std, scaling: sc, postsolve_log, orig_of_kept, sign, fixed_values, shift } = match build_std_form_presolved(variables, objective, constraints, !opts.distinguish_infeasible_unbounded) {
        Ok(pf) => pf,
        Err(status) => {
            crate::phase_timing::mark(if status == Status::Infeasible { "presolve_infeasible" } else { "presolve_no_finite_optimum" });
            return SimplexResult { status, x: None };
        }
    };
    crate::phase_timing::mark("presolve_end");
    if env_str!("ENOMOTO_DEBUG_PRESOLVE_SIZE").is_some() {
        // n_vars_out は求解に渡る構造列数 (固定変数は除外済み)。
        eprintln!(
            "PRESOLVE_SIZE n_vars_in={} n_rows_in={} n_vars_out={} n_rows_out={}",
            variables.len(),
            constraints.len(),
            std.n_total - std.n_rows,
            std.n_rows
        );
    }
    if env_str!("ENOMOTO_PRESOLVE_ONLY").is_some() {
        // 前処理後の大きさだけを測る (ベンチマークの対象選び用、`ENOMOTO_DEBUG_PRESOLVE_SIZE` と併用)。
        return SimplexResult { status: Status::NotSolved, x: None };
    }
    if env_str!("ENOMOTO_DEBUG_EXT_COMPONENTS").is_some() {
        match connected_components_of_std_form(&std) {
            Some((components, _has_row)) => {
                let mut sizes: Vec<usize> = components.iter().map(|c| c.len()).collect();
                sizes.sort_unstable_by(|a, b| b.cmp(a));
                eprintln!("DEBUG_EXT_COMPONENTS: n_components={} sizes={:?}", components.len(), sizes);
            }
            None => eprintln!("DEBUG_EXT_COMPONENTS: single component (no split found)"),
        }
    }
    // 試験用 (`ENOMOTO_T_POST_SCALE`): 前処理 (行・列の削除) の後でもう一度行・列をそろえる
    // (1: Ruiz、2: Ruiz + Pock–Chambolle、3: Pock–Chambolle)。双対二段解法と内点法 + クロスオーバーの両方に効く。
    let (std, post_dc) = match post_scale(&std, tunable!("ENOMOTO_T_POST_SCALE", 0u8, u8)) {
        Some((s2, dc)) => (s2, Some(dc)),
        None => (std, None),
    };
    let race_min_rows = tunable!("ENOMOTO_T_RACE_MIN_ROWS", RACE_MIN_ROWS, usize);
    let (result, std) = if opts.auto_race && std.n_rows >= race_min_rows {
        // 大きな問題: 傾き・切片双対二段解法と内点法 + クロスオーバーを同時に解き、先に結論を出した側を採る。
        let std = std::sync::Arc::new(std);
        (race::solve_race(std.clone(), opts), None)
    } else {
        (solve_one_engine(&std, &opts), Some(std))
    };
    drop(std);
    crate::phase_timing::mark("simplex_end");
    if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
        eprintln!("DEBUG_EXT: solve_std_form_decomposed returned {:?}", result.status);
    }
    let mut result = result;
    if let (Some(dc), Some(x)) = (&post_dc, result.x.as_mut()) {
        for (v, d) in x.iter_mut().zip(dc) {
            *v *= d;
        }
    }
    unscale_result(result, &sc, &postsolve_log, &orig_of_kept, &sign, &fixed_values, &shift, variables.len())
}

/// 前処理後の標準形の行・列をそろえ直す (`mode` 1: Ruiz、2: Ruiz + Pock–Chambolle、3: Pock–Chambolle、0: しない)。
/// 縮尺は構造列だけで求め、スラック列は `1 / r_i` 倍して単位列のまま保つ (境界は `r_i` 倍)。
/// 戻り値はそろえた標準形と構造列の縮尺 `dc` (`x = dc ∘ x'`)。
fn post_scale(std: &StdForm, mode: u8) -> Option<(StdForm, Vec<f64>)> {
    if mode == 0 {
        return None;
    }
    let m = std.n_rows;
    let n = std.n_total;
    let ns = n - m;
    let mut rows: Vec<Vec<(usize, f64)>> = (0..m).map(|i| std.rows.row(i).iter().copied().filter(|&(j, _)| j < ns).collect()).collect();
    let mut dr = vec![1.0f64; m];
    let mut dc = vec![1.0f64; ns];
    let mut apply = |rows: &mut Vec<Vec<(usize, f64)>>, rs: &[f64], cs: &[f64], dr: &mut [f64], dc: &mut [f64]| {
        for (i, r) in rows.iter_mut().enumerate() {
            for e in r.iter_mut() {
                e.1 *= rs[i] * cs[e.0];
            }
            dr[i] *= rs[i];
        }
        for j in 0..ns {
            dc[j] *= cs[j];
        }
    };
    let inv_sqrt = |v: f64| if v > 0.0 && v.is_finite() { 1.0 / v.sqrt() } else { 1.0 };
    if mode == 1 || mode == 2 {
        for _ in 0..tunable!("ENOMOTO_T_POST_SCALE_RUIZ_ITERS", 10usize, usize) {
            let mut rmax = vec![0.0f64; m];
            let mut cmax = vec![0.0f64; ns];
            for (i, r) in rows.iter().enumerate() {
                for &(j, v) in r {
                    rmax[i] = rmax[i].max(v.abs());
                    cmax[j] = cmax[j].max(v.abs());
                }
            }
            let rs: Vec<f64> = rmax.iter().map(|&v| inv_sqrt(v)).collect();
            let cs: Vec<f64> = cmax.iter().map(|&v| inv_sqrt(v)).collect();
            apply(&mut rows, &rs, &cs, &mut dr, &mut dc);
        }
    }
    if mode == 2 || mode == 3 {
        let mut rsum = vec![0.0f64; m];
        let mut csum = vec![0.0f64; ns];
        for (i, r) in rows.iter().enumerate() {
            for &(j, v) in r {
                rsum[i] += v.abs();
                csum[j] += v.abs();
            }
        }
        let rs: Vec<f64> = rsum.iter().map(|&v| inv_sqrt(v)).collect();
        let cs: Vec<f64> = csum.iter().map(|&v| inv_sqrt(v)).collect();
        apply(&mut rows, &rs, &cs, &mut dr, &mut dc);
    }
    for (i, r) in rows.iter_mut().enumerate() {
        r.push((ns + i, 1.0));
    }
    let (rows_m, cols_m) = freeze_std_matrices(&rows, n);
    let mut c = std.c.clone();
    let mut lb = std.lb.clone();
    let mut ub = std.ub.clone();
    for j in 0..ns {
        c[j] *= dc[j];
        lb[j] /= dc[j];
        ub[j] /= dc[j];
    }
    for i in 0..m {
        // スラック: 元の行 i の `a x + s = b` を r_i 倍すると `a' x' + r_i s = r_i b`、`s' = r_i s`。
        lb[ns + i] *= dr[i];
        ub[ns + i] *= dr[i];
    }
    let b = (0..m).map(|i| std.b[i] * dr[i]).collect();
    Some((StdForm { n_total: n, n_rows: m, c, rows: rows_m, cols: cols_m, b, lb, ub }, dc))
}

/// 前処理後の標準形を、`opts` で選ばれた 1 つのエンジンで解く (同時実行しない場合)。
fn solve_one_engine(std: &StdForm, opts: &crate::types::LpOptions) -> SimplexResult {
    if opts.ipm_crossover && tunable!("ENOMOTO_T_IPM_STAGED", 0u8, u8) != 0 {
        return solve_staged_ipm(std, opts);
    }
    if opts.ipm_crossover {
        // 小さな問題 (非零の数が `ENOMOTO_T_XO_SERIAL_NNZ` 未満) は 1 スレッドのプールで解く: 内点法は 1 反復に何十回も
        // 並列ループを呼び、小さな問題では眠ったスレッドを起こす待ちが計算より長い (blend: 1 反復 1.6 ミリ秒、
        // 計測ごとに 4〜34 ミリ秒とばらつく)。
        let serial_nnz = tunable!("ENOMOTO_T_XO_SERIAL_NNZ", XO_SERIAL_NNZ, usize);
        if std.cols.nnz() < serial_nnz && rayon::current_num_threads() > 1 {
            static POOL: std::sync::OnceLock<Option<rayon::ThreadPool>> = std::sync::OnceLock::new();
            if let Some(pool) = POOL.get_or_init(|| rayon::ThreadPoolBuilder::new().num_threads(1).build().ok()) {
                return pool.install(|| solve_ipm_crossover_engine(std, opts));
            }
        }
        solve_ipm_crossover_engine(std, opts)
    } else {
        solve_std_form_decomposed(std, opts)
    }
}

/// 内点法 + クロスオーバー ([`solve_one_engine`] の本体)。
fn solve_ipm_crossover_engine(std: &StdForm, opts: &crate::types::LpOptions) -> SimplexResult {
    crate::phase_timing::mark("xo_engine_start");
    {
        // 内点法 + クロスオーバー。内点法が収束しない・基底が作れないときは傾き・切片二段解法で解き直す。
        // 大きな独立成分が 2 つ以上あれば成分ごとに分けて解く。
        if let Some(r) = solve_ipm_crossover_split(std, opts) {
            return r;
        }
        crossover::solve_ipm_crossover(std).unwrap_or_else(|| {
            crate::phase_timing::mark("crossover_fallback");
            solve_std_form_decomposed(std, opts)
        })
    }
}

/// 段階 A を傾き・切片二段解法 (双対単体法) で解き、段階 B に当たる部分を内点法で解く (試験用
/// `ENOMOTO_T_IPM_STAGED=1`)。段階 A の結果で、内点法が結論すべきことは 2 択に絞られる:
///
/// - `z^1 = 0` (双対実行可能): 内点法 + クロスオーバーで元の問題を解く。段階 A の双対を近接中心にし
///   (`ENOMOTO_T_IPM_STAGED_CENTER=0` で使わない)、内点法は「最適」か「双対の発散 (実行不能)」だけを結論する。
/// - `z^1 < 0` (有限最適なし): 費用 0 の実行可能性問題を内点法で解き、実行可能なら非有界 (段階 A の傾き
///   `x^1` が半直線)、双対が発散すれば実行不能。
///
/// 内点法が結論できなければ二段解法で一から解き直す。
fn solve_staged_ipm(std: &StdForm, opts: &crate::types::LpOptions) -> SimplexResult {
    let fallback = || {
        crate::phase_timing::mark("crossover_fallback");
        solve_std_form_decomposed(std, opts)
    };
    match slope_intercept_dual::solve_stage_a(std, opts) {
        Some(slope_intercept_dual::StageA::Done(r)) if r.status != Status::NotSolved => r,
        Some(slope_intercept_dual::StageA::DualFeasible { y }) => {
            crate::phase_timing::mark("staged_stage_a_dual_feasible");
            let center = tunable!("ENOMOTO_T_IPM_STAGED_CENTER", 1u8, u8) != 0;
            let xo = crossover::XoOptions { dual_center: center.then_some(&y[..]), dual_feasible_known: true, ..Default::default() };
            crossover::solve_ipm_crossover_with(std, &xo).unwrap_or_else(fallback)
        }
        Some(slope_intercept_dual::StageA::NoFiniteOptimum { .. }) => {
            crate::phase_timing::mark("staged_stage_a_no_finite");
            match crossover::ipm_feasibility(std) {
                Some(true) => SimplexResult { status: Status::Unbounded, x: None },
                Some(false) => SimplexResult { status: Status::Infeasible, x: None },
                None => fallback(),
            }
        }
        _ => fallback(),
    }
}

/// 単体法モジュールのテスト。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LinearExpr, VarType};
    use std::collections::BTreeMap;

    /// rayon と逐次ループの速度比較 (正しさのテストではない診断用、`#[ignore]`)。
    /// `cargo test --release rayon_threshold_microbench -- --ignored --nocapture` で実行する。
    #[test]
    #[ignore]
    fn rayon_threshold_microbench() {
        use rayon::prelude::*;
        use std::time::Instant;

        /// chuzr 風の 1 要素分の仕事 (違反量の二乗 / 重み)。
        fn work(i: usize, data: &[f64]) -> Option<(usize, f64)> {
            let v = data[i];
            let delta = if v < 0.3 { 0.3 - v } else { 0.0 };
            if delta <= 1e-9 {
                return None;
            }
            let score = delta * delta / (data[(i + 1) % data.len()]).max(1e-9);
            Some((i, score))
        }

        for &n in &[300usize, 1_000, 5_000, 10_000, 50_000, 200_000] {
            let data: Vec<f64> = (0..n).map(|i| ((i * 2654435761u64 as usize) % 1000) as f64 / 1000.0).collect();
            /// 計測の繰り返し回数。
            const REPS: usize = 200;

            let t0 = Instant::now();
            for _ in 0..REPS {
                let _best = (0..n).into_iter().filter_map(|i| work(i, &data)).max_by(|a, b| a.1.total_cmp(&b.1));
            }
            let seq = t0.elapsed() / REPS as u32;

            let t1 = Instant::now();
            for _ in 0..REPS {
                let _best = (0..n).into_par_iter().filter_map(|i| work(i, &data)).max_by(|a, b| a.1.total_cmp(&b.1));
            }
            let par = t1.elapsed() / REPS as u32;

            println!("n={n:>7} sequential={seq:>10?} rayon={par:>10?} rayon/sequential={:.2}x", par.as_secs_f64() / seq.as_secs_f64());
        }
    }

    /// `DseState::update_after_pivot` と同じ要素ごとの更新での rayon/逐次の速度比較 (診断用、`#[ignore]`)。
    #[test]
    #[ignore]
    fn dse_update_rayon_threshold_microbench() {
        use rayon::prelude::*;
        use std::time::Instant;

        for &m in &[300usize, 1_000, 5_000, 10_000, 50_000, 200_000] {
            let alpha: Vec<f64> = (0..m).map(|i| ((i * 2654435761u64 as usize) % 1000) as f64 / 1000.0 + 0.1).collect();
            let tau: Vec<f64> = (0..m).map(|i| ((i * 40503u64 as usize) % 1000) as f64 / 1000.0).collect();
            let w: Vec<f64> = vec![1.0; m];
            let p = m / 2;
            let pivot = alpha[p];
            let wp_old = w[p];
            let update_one = |i: usize, w_i: &mut f64| {
                if i == p {
                    return;
                }
                let ratio = alpha[i] / pivot;
                *w_i = (*w_i - 2.0 * ratio * tau[i] + ratio * ratio * wp_old).max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
            };
            /// 計測の繰り返し回数。
            const REPS: usize = 200;

            let t0 = Instant::now();
            for _ in 0..REPS {
                let mut seq_w = w.clone();
                for (i, w_i) in seq_w.iter_mut().enumerate() {
                    update_one(i, w_i);
                }
                std::hint::black_box(&seq_w);
            }
            let seq = t0.elapsed() / REPS as u32;

            let t1 = Instant::now();
            for _ in 0..REPS {
                let mut par_w = w.clone();
                par_w.par_iter_mut().enumerate().for_each(|(i, w_i)| update_one(i, w_i));
                std::hint::black_box(&par_w);
            }
            let par = t1.elapsed() / REPS as u32;

            println!("m={m:>7} sequential={seq:>10?} rayon={par:>10?} rayon/sequential={:.2}x", par.as_secs_f64() / seq.as_secs_f64());
        }
    }

    /// 連続変数 `[lb, ub]` を作る。
    fn var(lb: f64, ub: f64) -> VariableData {
        VariableData { vtype: VarType::Continuous, lb, ub }
    }

    /// 係数リストから定数項 0 の線形式を作る。
    fn expr(terms: &[(usize, f64)]) -> LinearExpr {
        LinearExpr { coeffs: terms.iter().cloned().collect::<BTreeMap<_, _>>(), constant: 0.0 }
    }

    /// 制約行 `terms (sense) rhs` を作る。
    fn row(terms: &[(usize, f64)], sense: RowSense, rhs: f64) -> ConstraintRow {
        ConstraintRow { expr: expr(terms), sense, rhs }
    }

    /// 絶対誤差 1e-6 以内で等しいか。
    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    /// 内点法 (IP-PMM) で直接解く (このモジュールと独立な照合用)。
    fn solve_via_ipm(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow]) -> crate::types::SolveResult {
        crate::solver::solve_lp(variables, objective, constraints, crate::types::RootSolver::Interior, crate::types::LpOptions::default())
    }

    /// max x + 2y s.t. x+y<=10, x,y∈[0,10] → 最適 (0,10)。
    #[test]
    fn lp1_maximize_with_le_bounds() {
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 2.0)]), sense: Sense::Maximize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 10.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 0.0), "x={x:?}");
        assert!(approx(x[1], 10.0), "x={x:?}");
    }

    /// min a+b s.t. a+2b>=6, a-b==0, a,b∈[0,1000] → 最適 (2,2) (上限は効かない)。
    #[test]
    fn lp2_minimize_with_ge_and_eq() {
        let vars = vec![var(0.0, 1000.0), var(0.0, 1000.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 2.0)], RowSense::Ge, 6.0),
            row(&[(0, 1.0), (1, -1.0)], RowSense::Eq, 0.0),
        ];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 2.0), "x={x:?}");
        assert!(approx(x[1], 2.0), "x={x:?}");
    }

    /// min z s.t. z>=-5, z<=100, z∈[-1e6,1e6] → 最適 -5 (変数自身の上下限は効かない)。
    #[test]
    fn lp3_wide_bounds_dont_bind() {
        let vars = vec![var(-1.0e6, 1.0e6)];
        let obj = Objective { expr: expr(&[(0, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0)], RowSense::Ge, -5.0),
            row(&[(0, 1.0)], RowSense::Le, 100.0),
        ];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        assert!(approx(res.x.unwrap()[0], -5.0));
    }

    /// w∈[0,5], w>=10 → 実行不能。
    #[test]
    fn infeasible_detected() {
        let vars = vec![var(0.0, 5.0)];
        let obj = Objective { expr: expr(&[(0, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0)], RowSense::Ge, 10.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Infeasible);
    }

    /// min Σ(i+1)x_i s.t. Σx_i == 5, x_i∈[0,1] (1 行)。BFRT が 1 反復で x0..x3 をフリップし
    /// x4 を入れて貪欲解 (x0..x4 = 1) に到達することを確認する。BFRT なしでは x0 が
    /// 上限 1 を超えて 5 まで動いてしまう (正しさのための機構であることの確認)。
    #[test]
    fn dual_bfrt_flips_multiple_variables_in_one_iteration() {
        let n = 10;
        let vars: Vec<VariableData> = (0..n).map(|_| var(0.0, 1.0)).collect();
        let obj = Objective { expr: expr(&(0..n).map(|i| (i, (i + 1) as f64)).collect::<Vec<_>>()), sense: Sense::Minimize };
        let cons = vec![row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Eq, 5.0)];

        let expected_x = [1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let check = |x: Vec<f64>| {
            let objective: f64 = obj.expr.coeffs.iter().map(|(&j, &c)| c * x[j]).sum();
            assert!(approx(objective, 15.0), "objective={objective}, x={x:?}");
            for j in 0..n {
                assert!(approx(x[j], expected_x[j]), "x[{j}]={}, x={x:?}", x[j]);
            }
        };

        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());

        let ipm = solve_via_ipm(&vars, &obj, &cons);
        assert_eq!(ipm.status, Status::Optimal);
        check(ipm.x.unwrap());
    }

    /// 平行な不等式行 x+y<=10 と 2x+2y<=30 のうち、きつい方が残ること
    /// (`redundancy::reduce_inequalities`) を確認する。正解は x=10, y=0。
    #[test]
    fn parallel_inequality_rows_keep_the_tighter_one() {
        let vars = vec![var(0.0, 20.0), var(0.0, 20.0)];
        let obj = Objective { expr: expr(&[(0, 2.0), (1, 1.0)]), sense: Sense::Maximize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 10.0),
            row(&[(0, 2.0), (1, 2.0)], RowSense::Le, 30.0),
        ];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 10.0), "x={x:?}");
        assert!(approx(x[1], 0.0), "x={x:?}");
    }

    /// min z + x s.t. z+x<=10, z∈[2,8], x∈[0,10]。z は下向きロックがなく dualfix で下限 2 に
    /// 固定されうる。誤った固定なら目的値が狂う。正解は z=2, x=0。
    #[test]
    fn dualfix_fixes_a_dominated_variable() {
        let vars = vec![var(2.0, 8.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 10.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 2.0), "x={x:?}");
        assert!(approx(x[1], 0.0), "x={x:?}");
    }

    /// 列シングルトン x0 (x0 - x1 - x2 == 0 にだけ現れる) の代入と費用の畳み込みを確認する。
    /// min -3x0 + x1 + 2x2, x1+x2<=10 → 正解 x0=10, x1=10, x2=0, 目的値 -20。IPM とも照合。
    #[test]
    fn colsingleton_substitutes_singleton_equality_and_folds_cost() {
        let vars = vec![var(0.0, 20.0), var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -3.0), (1, 1.0), (2, 2.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, -1.0), (2, -1.0)], RowSense::Eq, 0.0),
            row(&[(1, 1.0), (2, 1.0)], RowSense::Le, 10.0),
        ];
        let check = |x: Vec<f64>| {
            assert!(approx(x[0], 10.0), "x={x:?}");
            assert!(approx(x[1], 10.0), "x={x:?}");
            assert!(approx(x[2], 0.0), "x={x:?}");
            let objective: f64 = obj.expr.coeffs.iter().map(|(&j, &c)| c * x[j]).sum();
            assert!(approx(objective, -20.0), "objective={objective}, x={x:?}");
        };

        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());

        let ipm = solve_via_ipm(&vars, &obj, &cons);
        assert_eq!(ipm.status, Status::Optimal);
        check(ipm.x.unwrap());
    }

    /// 自由変数 x0 が 2 本の等式行 (x0+x1==5, x0-x2==1) を通じて `presolve::freevar` で
    /// 消去される経路を、求解全体で確認する。正解 x0=5, x1=0, x2=4。
    #[test]
    fn freevar_eliminated_through_multiple_equality_rows_end_to_end() {
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(1, 2.0), (2, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Eq, 5.0),
            row(&[(0, 1.0), (2, -1.0)], RowSense::Eq, 1.0),
        ];
        let check = |x: Vec<f64>| {
            assert!(approx(x[0], 5.0), "x={x:?}");
            assert!(approx(x[1], 0.0), "x={x:?}");
            assert!(approx(x[2], 4.0), "x={x:?}");
        };

        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());
    }

    /// どの行にも現れない自由変数 x0 に非零費用 → 非有界 (既定は InfeasibleOrUnbounded、
    /// 区別モードでは Unbounded)。
    #[test]
    fn freevar_leftover_unconstrained_with_nonzero_cost_is_unbounded() {
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(1, 1.0)], RowSense::Le, 5.0)];

        assert_eq!(solve_lp_dual(&vars, &obj, &cons).status, Status::InfeasibleOrUnbounded);
        let distinguish = crate::types::LpOptions { distinguish_infeasible_unbounded: true, ..Default::default() };
        assert_eq!(solve_lp_dual_with(&vars, &obj, &cons, distinguish).status, Status::Unbounded);
    }

    /// 改善半直線 (自由変数 x0) があっても残りが実行不能 (y1-y2>=1, y2-y3>=1, y3-y1>=1) なら、
    /// 区別モードでは Unbounded でなく Infeasible と報告すること。
    #[test]
    fn improving_ray_over_an_infeasible_rest_is_infeasible_when_distinguishing() {
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(0.0, f64::INFINITY), var(0.0, f64::INFINITY), var(0.0, f64::INFINITY)];
        let obj = Objective { expr: expr(&[(0, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(1, 1.0), (2, -1.0)], RowSense::Ge, 1.0),
            row(&[(2, 1.0), (3, -1.0)], RowSense::Ge, 1.0),
            row(&[(3, 1.0), (1, -1.0)], RowSense::Ge, 1.0),
        ];

        assert_eq!(solve_lp_dual(&vars, &obj, &cons).status, Status::InfeasibleOrUnbounded);
        let distinguish = crate::types::LpOptions { distinguish_infeasible_unbounded: true, ..Default::default() };
        assert_eq!(solve_lp_dual_with(&vars, &obj, &cons, distinguish).status, Status::Infeasible);
    }

    /// どの行にも現れない費用 0 の自由変数は 0 に固定され、他の変数の最適 (x1=3) は変わらない。
    #[test]
    fn freevar_leftover_unconstrained_with_zero_cost_fixes_to_zero() {
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(1, 1.0)], RowSense::Ge, 3.0)];

        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        let x = dual.x.unwrap();
        assert!(approx(x[0], 0.0), "x={x:?}");
        assert!(approx(x[1], 3.0), "x={x:?}");
    }

    /// 不等式行の対でのみ x0+x1=5, x0-x1=1 に拘束される両側無限の自由変数 2 つ
    /// (前処理で消せない残余ケース) を、分割せずに `slope_intercept_dual` が解けることを確認する。正解 (3,2)。
    #[test]
    fn freevar_residual_both_sides_infinite_only_in_inequality_rows_end_to_end() {
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(f64::NEG_INFINITY, f64::INFINITY)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 5.0),
            row(&[(0, 1.0), (1, 1.0)], RowSense::Ge, 5.0),
            row(&[(0, 1.0), (1, -1.0)], RowSense::Le, 1.0),
            row(&[(0, 1.0), (1, -1.0)], RowSense::Ge, 1.0),
        ];
        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        let x = dual.x.unwrap();
        assert!(approx(x[0], 3.0), "x={x:?}");
        assert!(approx(x[1], 2.0), "x={x:?}");
    }

    /// 自由変数 x0 が不等式行 2 本にだけ現れる場合 (前処理で片側の暗黙上下限が付く)。
    /// max x0 s.t. x0+x1<=8, x0-x1<=3 → 正解 x0=5.5, x1=2.5。
    #[test]
    fn freevar_residual_only_in_inequality_rows_with_nonzero_objective_end_to_end() {
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 8.0),
            row(&[(0, 1.0), (1, -1.0)], RowSense::Le, 3.0),
        ];
        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        let x = dual.x.unwrap();
        assert!(approx(x[0], 5.5), "x={x:?}");
        assert!(approx(x[1], 2.5), "x={x:?}");
    }

    /// 互いにだけ結合した 2 つの自由変数 (最適解は直線 x0 - x1 = -3)。`slope_intercept_dual` の
    /// cleanup が一方を `NbStatus::Zero` に置き、正しい目的値で終わることを確認する。
    #[test]
    fn mutually_coupled_free_variables_reach_a_correct_answer() {
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(f64::NEG_INFINITY, f64::INFINITY)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, -1.0)], RowSense::Le, 3.0), row(&[(0, -1.0), (1, 1.0)], RowSense::Le, 3.0)];
        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        let x = dual.x.unwrap();
        assert!(x[0].is_finite() && x[1].is_finite(), "x={x:?}");
        assert!(approx(x[0] - x[1], -3.0), "x={x:?}");
    }

    /// 不等式行 1 本 (x0+x1<=8) にだけ現れる自由変数 x0 は `presolve::freevar` が完全に消去し、
    /// 構造列として残らないこと。min -x0 → 正解 x0=8, x1=0。
    #[test]
    fn freevar_in_exactly_one_inequality_row_eliminated_by_presolve_end_to_end() {
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 8.0)];

        let pf = build_std_form_presolved(&vars, &obj, &cons, true).unwrap();
        assert!(!has_unbounded_structural(&pf), "x0 should be fully eliminated by presolve, never reaching a structural column at all");

        let check = |x: Vec<f64>| {
            assert!(approx(x[0], 8.0), "x={x:?}");
            assert!(approx(x[1], 0.0), "x={x:?}");
        };
        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());
    }

    /// 等式行 x0 == x1 で消去された自由変数 x0 の代入が、x0 を含む不等式行 x0+x2<=5 にも
    /// 畳み込まれることの回帰テスト。正解 x0=5, x1=5, x2=0 (目的値 -10)。
    #[test]
    fn freevar_eliminated_via_equality_row_folds_into_a_shared_inequality_row_end_to_end() {
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(1, -2.0), (2, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, -1.0)], RowSense::Eq, 0.0), row(&[(0, 1.0), (2, 1.0)], RowSense::Le, 5.0)];
        let check = |x: Vec<f64>| {
            assert!(approx(x[0], 5.0), "x={x:?}");
            assert!(approx(x[1], 5.0), "x={x:?}");
            assert!(approx(x[2], 0.0), "x={x:?}");
        };
        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());
    }

    /// 両側無限の自由列・片側無限列・有界列・等式行が混在するときの列番号の圧縮
    /// (`new_lb`/`new_ub`/スラック番号) を確認する。正解 (3, 2, 4, 0)。
    #[test]
    fn freevar_residual_mixed_with_ordinary_and_bounded_columns_end_to_end() {
        let vars = vec![
            var(f64::NEG_INFINITY, f64::INFINITY),
            var(f64::NEG_INFINITY, f64::INFINITY),
            var(0.0, f64::INFINITY),
            var(0.0, 10.0),
        ];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0), (2, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 5.0),
            row(&[(0, 1.0), (1, 1.0)], RowSense::Ge, 5.0),
            row(&[(0, 1.0), (1, -1.0)], RowSense::Le, 1.0),
            row(&[(0, 1.0), (1, -1.0)], RowSense::Ge, 1.0),
            row(&[(2, 1.0), (3, -1.0)], RowSense::Eq, 4.0),
        ];
        let check = |x: Vec<f64>| {
            assert!(approx(x[0], 3.0), "x={x:?}");
            assert!(approx(x[1], 2.0), "x={x:?}");
            assert!(approx(x[2], 4.0), "x={x:?}");
            assert!(approx(x[3], 0.0), "x={x:?}");
        };
        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());
    }

    /// 行に現れない x0∈[0,+inf) の費用が無限側を向く → 非有界 (前処理 → 標準形 → 傾き・切片二段解法の全経路)。
    #[test]
    fn solve_lp_dual_end_to_end_unbounded_structural_column() {
        let vars = vec![var(0.0, f64::INFINITY), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(1, 1.0)], RowSense::Le, 5.0)];
        assert_eq!(solve_lp_dual(&vars, &obj, &cons).status, Status::InfeasibleOrUnbounded);
        let distinguish = crate::types::LpOptions { distinguish_infeasible_unbounded: true, ..Default::default() };
        assert_eq!(solve_lp_dual_with(&vars, &obj, &cons, distinguish).status, Status::Unbounded);
    }

    /// x0∈(-inf,10] を反転 (`x0 = 10 - y0`) して解く経路を確認する。正解 x0=10。
    #[test]
    fn lb_unbounded_below_reaches_finite_optimum() {
        let vars = vec![var(f64::NEG_INFINITY, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 15.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 10.0), "x={x:?}");
        let objective: f64 = obj.expr.coeffs.iter().map(|(&j, &c)| c * x[j]).sum();
        assert!(approx(objective, -10.0), "objective={objective}, x={x:?}");
    }

    /// x0∈[0,+inf) が 2 本の 3 変数等式行 (x0<=8 と x0<=6 相当) で抑えられる → 正解 x0=6。
    #[test]
    fn solve_lp_dual_end_to_end_finite_optimum_with_unbounded_structural_column() {
        let vars = vec![var(0.0, f64::INFINITY), var(0.0, 10.0), var(0.0, 10.0), var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0), (2, 1.0)], RowSense::Eq, 8.0), row(&[(0, 1.0), (3, 1.0), (4, 1.0)], RowSense::Eq, 6.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 6.0), "x={x:?}");
        let objective: f64 = obj.expr.coeffs.iter().map(|(&j, &c)| c * x[j]).sum();
        assert!(approx(objective, -6.0), "objective={objective}, x={x:?}");
    }

    /// `pf.std` に無限の上下限を持つ構造列が残っているか。
    fn has_unbounded_structural(pf: &PresolvedForm) -> bool {
        let n_orig = pf.std.n_total - pf.std.n_rows;
        (0..n_orig).any(|j| pf.std.lb[j] == f64::NEG_INFINITY || pf.std.ub[j] == f64::INFINITY)
    }

    /// 前処理後も残る片側無限の上下限が、標準形で無限のまま残ること (有界問題では無限列なし)。
    #[test]
    fn surviving_one_sided_bounds_stay_infinite_in_std_form() {
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 10.0)];
        let pf = build_std_form_presolved(&vars, &obj, &cons, true).unwrap();
        assert!(!has_unbounded_structural(&pf));

        // 行に現れず費用が無限側を向く片側無限の変数は前処理を生き残り、ub は無限のまま。
        let vars = vec![var(0.0, f64::INFINITY), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(1, 1.0)], RowSense::Le, 5.0)];
        let pf = build_std_form_presolved(&vars, &obj, &cons, true).unwrap();
        assert!(has_unbounded_structural(&pf));
        assert_eq!(pf.std.ub[0], f64::INFINITY, "a surviving infinite bound must be left in place");
    }

    /// doubleton 等式 4x0 + 2x1 == 12 で係数の大きい x0 を消去し、x0 を含む別の行も書き換えること。
    /// 正解 x0=3, x1=0, x2=0 (目的値 -9)。IPM とも照合。
    #[test]
    fn doubleton_eliminates_larger_coefficient_variable_and_rewrites_other_row() {
        let vars = vec![var(0.0, 20.0), var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -3.0), (1, 1.0), (2, 2.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 4.0), (1, 2.0)], RowSense::Eq, 12.0),
            row(&[(0, 1.0), (2, 1.0)], RowSense::Le, 8.0),
            row(&[(1, 1.0), (2, 1.0)], RowSense::Le, 10.0),
        ];
        let check = |x: Vec<f64>| {
            assert!(approx(x[0], 3.0), "x={x:?}");
            assert!(approx(x[1], 0.0), "x={x:?}");
            assert!(approx(x[2], 0.0), "x={x:?}");
            let objective: f64 = obj.expr.coeffs.iter().map(|(&j, &c)| c * x[j]).sum();
            assert!(approx(objective, -9.0), "objective={objective}, x={x:?}");
        };

        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());

        let ipm = solve_via_ipm(&vars, &obj, &cons);
        assert_eq!(ipm.status, Status::Optimal);
        check(ipm.x.unwrap());
    }

    /// 同じパスで連鎖する 2 つの doubleton 消去 (x0 は x1 で、x1 は x2 で表される) の
    /// 復元順序を確認する。正解 x0=3, x1=0, x2=10 (目的値 -3)。IPM の後処理でも照合。
    #[test]
    fn doubleton_chain_within_one_pass_recovers_correctly() {
        let vars = vec![var(0.0, 20.0), var(0.0, 20.0), var(0.0, 20.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 4.0), (1, 2.0)], RowSense::Eq, 12.0),
            row(&[(1, 5.0), (2, 1.0)], RowSense::Eq, 10.0),
        ];
        let check = |x: Vec<f64>| {
            assert!(approx(x[0], 3.0), "x={x:?}");
            assert!(approx(x[1], 0.0), "x={x:?}");
            assert!(approx(x[2], 10.0), "x={x:?}");
            let objective: f64 = obj.expr.coeffs.iter().map(|(&j, &c)| c * x[j]).sum();
            assert!(approx(objective, -3.0), "objective={objective}, x={x:?}");
        };

        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());

        // 連鎖した doubleton 代入を IPM 側の後処理 (`unscale_with_substitutions`) でも確認する。
        let ipm = solve_via_ipm(&vars, &obj, &cons);
        assert_eq!(ipm.status, Status::Optimal);
        check(ipm.x.unwrap());
    }

    /// 重複/定数倍の等式行が前処理 (`redundancy::reduce_equalities`) で落とされること。正解 (2,2)。
    #[test]
    fn duplicate_equality_rows_via_shared_presolve() {
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Eq, 4.0),
            row(&[(0, 2.0), (1, 2.0)], RowSense::Eq, 8.0), // 上の行の定数倍
            row(&[(0, 1.0), (1, 1.0)], RowSense::Eq, 4.0), // 完全な重複
            row(&[(0, 1.0), (1, -1.0)], RowSense::Eq, 0.0),
        ];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 2.0), "x={x:?}");
        assert!(approx(x[1], 2.0), "x={x:?}");
    }

    /// 独立な 2 ブロック (変数 0,1 と 2,3,4) を連結成分分解で別々に解いて組み立てること。
    /// 正解 x = (0,4,10,5,0), 目的値 -36。
    #[test]
    fn disconnected_model_solves_via_connected_component_split() {
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0), var(0.0, 10.0), var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective {
            expr: expr(&[(0, 2.0), (1, 1.0), (2, -3.0), (3, -2.0), (4, -1.0)]),
            sense: Sense::Minimize,
        };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Ge, 4.0),
            row(&[(2, 1.0), (3, 1.0), (4, 1.0)], RowSense::Le, 15.0),
        ];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 0.0), "x={x:?}");
        assert!(approx(x[1], 4.0), "x={x:?}");
        assert!(approx(x[2], 10.0), "x={x:?}");
        assert!(approx(x[3], 5.0), "x={x:?}");
        assert!(approx(x[4], 0.0), "x={x:?}");
        let obj_val = 2.0 * x[0] + x[1] - 3.0 * x[2] - 2.0 * x[3] - x[4];
        assert!(approx(obj_val, -36.0), "obj={obj_val}");
    }

    /// 一方の成分が実行不能なら、他方が実行可能でも全体は Infeasible と報告すること。
    #[test]
    fn disconnected_model_reports_infeasible_when_either_component_is() {
        let vars = vec![var(0.0, 1.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0)], RowSense::Ge, 5.0), row(&[(1, 1.0)], RowSense::Le, 10.0)];
        assert_eq!(solve_lp_dual(&vars, &obj, &cons).status, Status::Infeasible);
    }

    /// 250 個の独立な 1 変数ブロック (各 x_i <= (i%7)+1) を解き、各最適値を確認する。
    #[test]
    fn many_independent_singleton_components_exercise_the_parallel_split_path() {
        /// ブロック (変数) の数。
        const N: usize = 250;
        let vars: Vec<VariableData> = (0..N).map(|_| var(0.0, 20.0)).collect();
        let obj = Objective { expr: expr(&(0..N).map(|i| (i, -1.0)).collect::<Vec<_>>()), sense: Sense::Minimize };
        let cons: Vec<ConstraintRow> = (0..N).map(|i| row(&[(i, 1.0)], RowSense::Le, ((i % 7) + 1) as f64)).collect();
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        for i in 0..N {
            let expected = ((i % 7) + 1) as f64;
            assert!(approx(x[i], expected), "x[{i}]={} expected={expected}", x[i]);
        }
    }

    /// 前処理を通さず直接作る、各 `k` 変数の独立な 2 ブロックの標準形。ブロック `b` は
    /// `min -Σ (j+1) x_j  s.t. Σ x_j <= caps[b], x∈[0,1]` (単位重みの連続ナップサック)。
    fn two_block_knapsack_std_form(k: usize, caps: [f64; 2]) -> StdForm {
        let n_orig = 2 * k;
        let n_total = n_orig + 2;
        let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
        for b in 0..2 {
            let mut row: Vec<(usize, f64)> = (0..k).map(|j| (b * k + j, 1.0)).collect();
            row.push((n_orig + b, 1.0));
            rows.push(row);
        }
        let (rows, cols) = freeze_std_matrices(&rows, n_total);
        let mut c: Vec<f64> = (0..n_orig).map(|j| -((j % k) as f64 + 1.0)).collect();
        c.extend([0.0, 0.0]);
        let lb = vec![0.0; n_total];
        let mut ub = vec![1.0; n_orig];
        ub.extend([f64::INFINITY, f64::INFINITY]);
        StdForm { n_total, n_rows: 2, c, rows, cols, b: caps.to_vec(), lb, ub }
    }

    /// 各 250 変数の 2 ブロック (`PARALLEL_COMPONENT_MIN_VARS` 以上) を rayon 並列の傾き・切片二段解法で解くこと。
    #[test]
    fn large_independent_components_are_solved_by_parallel_slope_intercept_calls() {
        /// 1 ブロックあたりの変数数。
        const K: usize = 250;
        assert!(K >= PARALLEL_COMPONENT_MIN_VARS);
        let std = two_block_knapsack_std_form(K, [100.5, 30.25]);
        let (components, has_row) = connected_components_of_std_form(&std).expect("two blocks");
        assert_eq!(components.iter().filter(|c| c.len() > 1 || has_row[c[0]]).count(), 2);

        let res = solve_std_form_decomposed(&std, &crate::types::LpOptions::default());
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert_eq!(x.len(), 2 * K);
        for (b, cap) in [(0usize, 100.5f64), (1, 30.25)] {
            let whole = cap.floor() as usize;
            for j in 0..K {
                // ブロック内で添字が大きいほど費用 (の絶対値) が大きい。
                let rank = K - 1 - j;
                let expected = if rank < whole { 1.0 } else if rank == whole { cap - whole as f64 } else { 0.0 };
                assert!(approx(x[b * K + j], expected), "block {b} x[{j}]={} expected={expected}", x[b * K + j]);
            }
        }
    }

    /// [`combine_component_statuses`] の状態合成規則を確認する。
    #[test]
    fn component_statuses_combine_soundly() {
        use Status::*;
        let combine = |v: &[Status]| combine_component_statuses(v.iter());
        assert_eq!(combine(&[Optimal, Optimal]), Optimal);
        assert_eq!(combine(&[Optimal, Infeasible]), Infeasible);
        assert_eq!(combine(&[Unbounded, Infeasible]), Infeasible);
        assert_eq!(combine(&[NotSolved, Infeasible]), Infeasible);
        assert_eq!(combine(&[Optimal, Unbounded]), Unbounded);
        // 未解決の成分があると Unbounded とは言えないが、有限最適解なしは言える。
        assert_eq!(combine(&[Unbounded, NotSolved]), InfeasibleOrUnbounded);
        assert_eq!(combine(&[InfeasibleOrUnbounded, NotSolved]), InfeasibleOrUnbounded);
        assert_eq!(combine(&[InfeasibleOrUnbounded, Unbounded]), InfeasibleOrUnbounded);
        assert_eq!(combine(&[Optimal, NotSolved]), NotSolved);
    }

    /// 矛盾する等式行 (x+y==4 と x+y==5) は前処理ではなくソルバが実行不能と検出すること。
    #[test]
    fn contradictory_equality_rows_detected_infeasible() {
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Eq, 4.0),
            row(&[(0, 1.0), (1, 1.0)], RowSense::Eq, 5.0),
        ];
        assert_eq!(solve_lp_dual(&vars, &obj, &cons).status, Status::Infeasible);
    }

    /// 単一変数行 w>=10 が変数自身の上限 5 と矛盾する場合、伝播の整合性検査で実行不能とされること。
    #[test]
    fn single_variable_ge_row_conflicting_with_own_bound_is_infeasible() {
        let vars = vec![var(0.0, 5.0)];
        let obj = Objective { expr: expr(&[(0, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0)], RowSense::Ge, 10.0)];
        assert_eq!(solve_lp_dual(&vars, &obj, &cons).status, Status::Infeasible);
    }

    /// 0/1 ナップサックの LP 緩和 (最適値 240、整数最適 220 より良い)。
    #[test]
    fn knapsack_lp_relaxation() {
        let vars = vec![var(0.0, 1.0), var(0.0, 1.0), var(0.0, 1.0)];
        let obj = Objective { expr: expr(&[(0, 60.0), (1, 100.0), (2, 120.0)]), sense: Sense::Maximize };
        let cons = vec![row(&[(0, 10.0), (1, 20.0), (2, 30.0)], RowSense::Le, 50.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        let obj_val = 60.0 * x[0] + 100.0 * x[1] + 120.0 * x[2];
        // LP 緩和の最適値は 240 (x2=x3=1, x1=0)。
        assert!(approx(obj_val, 240.0), "obj={obj_val} x={x:?}");
    }

    /// 60 変数の連鎖制約 LP (FT_CHECK_INTERVAL を超える基底変更で周期検査を通る) の目的値を
    /// 独立な IPM ソルバと照合する。
    #[test]
    fn larger_lp_matches_independent_ipm_solver_and_exercises_ft_triggers() {
        let n = 60;
        let vars: Vec<VariableData> = (0..n).map(|_| var(0.0, 8.0)).collect();
        let obj_terms: Vec<(usize, f64)> = (0..n).map(|i| (i, 1.0 + (i % 4) as f64)).collect();
        let obj = Objective { expr: expr(&obj_terms), sense: Sense::Maximize };

        let mut cons: Vec<ConstraintRow> = Vec::new();
        for i in 0..(n - 1) {
            cons.push(row(&[(i, 1.0), (i + 1, 1.0)], RowSense::Le, 10.0));
        }
        cons.push(row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Le, 220.0));
        // スラック基底からは実行不能になる行 (第 1 段階の仕事を発生させる)。
        cons.push(row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Ge, 40.0));

        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        let obj_val: f64 = obj_terms.iter().map(|&(j, c)| c * x[j]).sum();

        let ipm_res = solve_via_ipm(&vars, &obj, &cons);
        assert_eq!(ipm_res.status, Status::Optimal);
        let ipm_obj = ipm_res.objective.unwrap();

        assert!((obj_val - ipm_obj).abs() < 1e-4, "simplex_obj={obj_val} ipm_obj={ipm_obj} x={x:?}");
    }

    /// `SteepestEdgeState::update_after_pivot` の差分更新が、ピボット後の基底を新たに分解して
    /// 直接計算した `||B_new^-1 A_j||^2` と一致することを確認する (手で追える非退化ピボット 1 回)。
    #[test]
    fn steepest_edge_weights_match_brute_force_recompute() {
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0), (2, 1.0)]), sense: Sense::Maximize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0), (2, 1.0)], RowSense::Le, 10.0),
            row(&[(0, 1.0), (1, -1.0), (2, 2.0)], RowSense::Le, 8.0),
        ];
        let std = build_std_form(&vars, &obj, &cons);
        let mut t = Tableau::new(&std);
        let lu = refactorize(&std, &t, None);
        t.recompute_basics(&lu);
        let mut se = SteepestEdgeState::new(&std);

        let enter = 0usize; // x0 が入る
        let a_enter = t.column(enter);
        let alpha = lu.solve(&a_enter);
        let r = 1usize; // 行 1 (s1) が theta=8 で先にブロックする

        let mut e_r = vec![0.0; std.n_rows];
        e_r[r] = 1.0;
        let rho = lu.solve_transpose(&e_r);
        let w = lu.solve_transpose(&alpha);
        let gamma_t_old = se.gamma[enter];
        let pivot = alpha[r];

        // run_phase と同じ手順でピボットを適用する (s1 は下限 0 で出る)。
        let leaving_var = t.basis[r];
        t.nb_status[leaving_var] = Some(NbStatus::Lower);
        t.basis_pos[leaving_var] = None;
        t.basis[r] = enter;
        t.basis_pos[enter] = Some(r);
        t.nb_status[enter] = None;

        se.update_after_pivot(&t, &std, &rho, &w, gamma_t_old, pivot);

        // 新しい基底を分解し直し、全非基底列で ||B_new^-1 A_j||^2 を直接計算して比較する。
        let fresh_lu = refactorize(&std, &t, None);
        for j in 0..std.n_total {
            if t.nb_status[j].is_none() {
                continue;
            }
            let col = t.column(j);
            let brute = fresh_lu.solve(&col);
            let brute_norm_sq: f64 = brute.iter().map(|v| v * v).sum();
            assert!(
                (se.gamma[j] - brute_norm_sq).abs() < 1e-6 * brute_norm_sq.max(1.0),
                "j={j} incremental={} brute_force={}",
                se.gamma[j],
                brute_norm_sq
            );
        }
    }

    /// lp1 と同じ LP。双対クラッシュが x=y=10 (主実行不能・双対実行可能) から始まる本物の双対ピボット。
    #[test]
    fn dual_lp1_matches_primal() {
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 2.0)]), sense: Sense::Maximize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 10.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 0.0), "x={x:?}");
        assert!(approx(x[1], 10.0), "x={x:?}");
    }

    /// lp2 と同じ LP を双対法で解く。
    #[test]
    fn dual_lp2_matches_primal() {
        let vars = vec![var(0.0, 1000.0), var(0.0, 1000.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 2.0)], RowSense::Ge, 6.0),
            row(&[(0, 1.0), (1, -1.0)], RowSense::Eq, 0.0),
        ];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 2.0), "x={x:?}");
        assert!(approx(x[1], 2.0), "x={x:?}");
    }

    /// ナップサック LP 緩和を双対法で解く (最適値 240)。
    #[test]
    fn dual_knapsack_lp_relaxation() {
        let vars = vec![var(0.0, 1.0), var(0.0, 1.0), var(0.0, 1.0)];
        let obj = Objective { expr: expr(&[(0, 60.0), (1, 100.0), (2, 120.0)]), sense: Sense::Maximize };
        let cons = vec![row(&[(0, 10.0), (1, 20.0), (2, 30.0)], RowSense::Le, 50.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        let obj_val = 60.0 * x[0] + 100.0 * x[1] + 120.0 * x[2];
        assert!(approx(obj_val, 240.0), "obj={obj_val} x={x:?}");
    }

    /// `larger_lp_matches_independent_ipm_solver_and_exercises_ft_triggers` と同じ LP を双対法で解き IPM と照合する。
    #[test]
    fn dual_larger_lp_matches_independent_ipm_solver() {
        let n = 60;
        let vars: Vec<VariableData> = (0..n).map(|_| var(0.0, 8.0)).collect();
        let obj_terms: Vec<(usize, f64)> = (0..n).map(|i| (i, 1.0 + (i % 4) as f64)).collect();
        let obj = Objective { expr: expr(&obj_terms), sense: Sense::Maximize };

        let mut cons: Vec<ConstraintRow> = Vec::new();
        for i in 0..(n - 1) {
            cons.push(row(&[(i, 1.0), (i + 1, 1.0)], RowSense::Le, 10.0));
        }
        cons.push(row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Le, 220.0));
        cons.push(row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Ge, 40.0));

        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        let obj_val: f64 = obj_terms.iter().map(|&(j, c)| c * x[j]).sum();

        let ipm_res = solve_via_ipm(&vars, &obj, &cons);
        assert_eq!(ipm_res.status, Status::Optimal);
        let ipm_obj = ipm_res.objective.unwrap();

        assert!((obj_val - ipm_obj).abs() < 1e-4, "dual_obj={obj_val} ipm_obj={ipm_obj} x={x:?}");
    }

    /// 内点法 + クロスオーバー (`LpOptions::ipm_crossover`) が単体法と同じ最適値を返し、内点法が
    /// 収束してクロスオーバーの全段 (`cleanup_end`) まで進むこと (単体法への解き直しでないこと)。
    #[test]
    fn ipm_crossover_matches_simplex() {
        let n = 60;
        let vars: Vec<VariableData> = (0..n).map(|_| var(0.0, 8.0)).collect();
        let obj_terms: Vec<(usize, f64)> = (0..n).map(|i| (i, 1.0 + (i % 4) as f64)).collect();
        let obj = Objective { expr: expr(&obj_terms), sense: Sense::Maximize };
        let mut cons: Vec<ConstraintRow> = Vec::new();
        for i in 0..(n - 1) {
            cons.push(row(&[(i, 1.0), (i + 1, 1.0)], RowSense::Le, 10.0));
        }
        cons.push(row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Le, 220.0));
        cons.push(row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Ge, 40.0));
        let res = solve_lp_dual(&vars, &obj, &cons);
        let x = res.x.unwrap();
        let ref_obj: f64 = obj_terms.iter().map(|&(j, c)| c * x[j]).sum();

        // クロスオーバーが (単体法への解き直しなしで) 結果を出すこと。
        let pf = build_std_form_presolved(&vars, &obj, &cons, true).ok().unwrap();
        assert!(crossover::solve_ipm_crossover(&pf.std).is_some_and(|r| r.status == Status::Optimal));
        let opts = crate::types::LpOptions { ipm_crossover: true, ..Default::default() };
        let res = solve_lp_dual_with(&vars, &obj, &cons, opts);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        let xo_obj: f64 = obj_terms.iter().map(|&(j, c)| c * x[j]).sum();
        assert!((ref_obj - xo_obj).abs() < 1e-7 * ref_obj.abs().max(1.0), "simplex={ref_obj} crossover={xo_obj}");
        for (i, c) in cons.iter().enumerate() {
            let lhs: f64 = c.expr.coeffs.iter().map(|(&j, &a)| a * x[j]).sum();
            let ok = match c.sense {
                RowSense::Le => lhs <= c.rhs + 1e-7,
                RowSense::Ge => lhs >= c.rhs - 1e-7,
                RowSense::Eq => (lhs - c.rhs).abs() <= 1e-7,
            };
            assert!(ok, "row {i} violated: lhs={lhs} rhs={}", c.rhs);
        }
    }

    /// 最適面が 1 点でない (主の最適解が一意でない) LP: 内点法は最適面の内部 (解析的中心) に収束するので、
    /// クロスオーバーの主の押し出しが頂点まで動かす必要がある。結果は頂点 (各変数が 0 か上限) になる。
    #[test]
    fn ipm_crossover_reaches_vertex_on_degenerate_optimal_face() {
        // max x0 + x1 + x2 + x3  s.t. x0 + x1 + x2 + x3 <= 2, 0 <= x <= 1 (最適値 2 の面は 4 次元の内部を持つ)
        let n = 4;
        let vars: Vec<VariableData> = (0..n).map(|_| var(0.0, 1.0)).collect();
        let terms: Vec<(usize, f64)> = (0..n).map(|i| (i, 1.0)).collect();
        let obj = Objective { expr: expr(&terms), sense: Sense::Maximize };
        let cons = vec![row(&terms, RowSense::Le, 2.0), row(&[(0, 1.0), (1, -1.0)], RowSense::Le, 0.5)];
        let opts = crate::types::LpOptions { ipm_crossover: true, ..Default::default() };
        let res = solve_lp_dual_with(&vars, &obj, &cons, opts);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!((x.iter().sum::<f64>() - 2.0).abs() < 1e-8, "x={x:?}");
        // 頂点: 4 変数・2 行なので、境界にない変数は高々 2 個。
        let interior = x.iter().filter(|&&v| v > 1e-8 && v < 1.0 - 1e-8).count();
        assert!(interior <= 2, "not a vertex: x={x:?}");
    }

    /// 同時実行 (`race::solve_race`) が単体法と同じ最適値を返すこと。実行不能な問題では、内点法 + クロスオーバー
    /// 側は結論を出さず (`None`)、二段解法側の `Infeasible` が採られること。
    #[test]
    fn staged_ipm_gives_the_same_verdicts_as_simplex() {
        let opts = crate::types::LpOptions { distinguish_infeasible_unbounded: true, ipm_crossover: true, auto_race: false };
        let n = 60;
        let vars: Vec<VariableData> = (0..n).map(|_| var(0.0, 8.0)).collect();
        let obj_terms: Vec<(usize, f64)> = (0..n).map(|i| (i, 1.0 + (i % 4) as f64)).collect();
        let obj = Objective { expr: expr(&obj_terms), sense: Sense::Maximize };
        let mut cons: Vec<ConstraintRow> = Vec::new();
        for i in 0..(n - 1) {
            cons.push(row(&[(i, 1.0), (i + 1, 1.0)], RowSense::Le, 10.0));
        }
        cons.push(row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Ge, 40.0));
        let x = solve_lp_dual(&vars, &obj, &cons).x.unwrap();
        let ref_obj: f64 = obj_terms.iter().map(|&(j, c)| c * x[j]).sum();
        let pf = build_std_form_presolved(&vars, &obj, &cons, false).ok().unwrap();
        let res = solve_staged_ipm(&pf.std, &opts);
        assert_eq!(res.status, Status::Optimal);
        let res = unscale_result(res, &pf.scaling, &pf.postsolve_log, &pf.orig_of_kept, &pf.sign, &pf.fixed_values, &pf.shift, n);
        let x = res.x.unwrap();
        let got: f64 = obj_terms.iter().map(|&(j, c)| c * x[j]).sum();
        assert!((got - ref_obj).abs() < 1e-7 * ref_obj.abs().max(1.0), "staged={got} simplex={ref_obj}");

        // 実行不能: 隣接 2 変数の和 <= 10 なのに全体の和 >= 400。
        let mut bad = cons.clone();
        bad.push(row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Ge, 400.0));
        if let Ok(pf) = build_std_form_presolved(&vars, &obj, &bad, false) {
            assert_eq!(solve_staged_ipm(&pf.std, &opts).status, Status::Infeasible);
        }

        // 非有界: 上限の無い変数を最大化 (隣接の和の制約を外し、x_k - x_{k+1} <= 1 の鎖にする)。
        let uvars: Vec<VariableData> = (0..n).map(|_| var(0.0, f64::INFINITY)).collect();
        let mut ucons: Vec<ConstraintRow> = Vec::new();
        for i in 0..(n - 1) {
            ucons.push(row(&[(i, 1.0), (i + 1, -1.0)], RowSense::Le, 1.0));
        }
        if let Ok(pf) = build_std_form_presolved(&uvars, &obj, &ucons, false) {
            assert_eq!(solve_staged_ipm(&pf.std, &opts).status, Status::Unbounded);
        }
        // 実行不能かつ非有界の可能性がある形: 非有界な目的と矛盾する制約。
        let mut ubad = ucons.clone();
        ubad.push(row(&[(0, 1.0)], RowSense::Le, -1.0));
        ubad.push(row(&[(1, 1.0), (2, 1.0)], RowSense::Ge, 3.0));
        if let Ok(pf) = build_std_form_presolved(&uvars, &obj, &ubad, false) {
            assert_eq!(solve_staged_ipm(&pf.std, &opts).status, Status::Infeasible);
        }
    }

    #[test]
    fn race_matches_simplex_and_keeps_infeasibility_verdict() {
        let n = 60;
        let vars: Vec<VariableData> = (0..n).map(|_| var(0.0, 8.0)).collect();
        let obj_terms: Vec<(usize, f64)> = (0..n).map(|i| (i, 1.0 + (i % 4) as f64)).collect();
        let obj = Objective { expr: expr(&obj_terms), sense: Sense::Maximize };
        let mut cons: Vec<ConstraintRow> = Vec::new();
        for i in 0..(n - 1) {
            cons.push(row(&[(i, 1.0), (i + 1, 1.0)], RowSense::Le, 10.0));
        }
        cons.push(row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Ge, 40.0));
        let x = solve_lp_dual(&vars, &obj, &cons).x.unwrap();
        let ref_obj: f64 = obj_terms.iter().map(|&(j, c)| c * x[j]).sum();
        let pf = build_std_form_presolved(&vars, &obj, &cons, false).ok().unwrap();
        let res = race::solve_race(std::sync::Arc::new(pf.std), Default::default());
        assert_eq!(res.status, Status::Optimal);
        let res = unscale_result(res, &pf.scaling, &pf.postsolve_log, &pf.orig_of_kept, &pf.sign, &pf.fixed_values, &pf.shift, n);
        let x = res.x.unwrap();
        let got: f64 = obj_terms.iter().map(|&(j, c)| c * x[j]).sum();
        assert!((got - ref_obj).abs() < 1e-7 * ref_obj.abs().max(1.0), "race={got} simplex={ref_obj}");

        // 実行不能: 隣接 2 変数の和 <= 10 なのに全体の和 >= 400。
        let mut bad = cons.clone();
        bad.push(row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Ge, 400.0));
        if let Ok(pf) = build_std_form_presolved(&vars, &obj, &bad, false) {
            let res = race::solve_race(std::sync::Arc::new(pf.std), Default::default());
            assert_eq!(res.status, Status::Infeasible);
        }
    }
}
