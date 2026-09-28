//! 制約伝播 (constraint propagation) による変数境界の強化。
//!
//! Achterberg, Bixby, Gu, Rothberg, Weninger, *"Presolve Reductions in
//! Mixed Integer Programming"* (ZIB Report 16-44) §3.1-3.2 に基づく。
//! 行 `sum_j a_ij x_j <= b_i` の最小/最大活動度 (activity)
//!
//!   inf{A_i. x} = sum_{a_ij>0} a_ij * lb_j + sum_{a_ij<0} a_ij * ub_j
//!   sup{A_i. x} = sum_{a_ij>0} a_ij * ub_j + sum_{a_ij<0} a_ij * lb_j
//!
//! を使い、
//! - `sup <= b + eps` なら行は冗長 (常に満たされる) なので削除、
//! - `inf > b + eps` なら問題は実行不能、
//! - `inf == b` なら強制行 (forcing row): 全変数を最小活動度側の境界に固定して行を削除、
//! - それ以外は「x_k を除いた最小活動度」から x_k の境界を締める (ZIB Report 16-44 §3.2)。
//!
//! このコードベースでは変数境界は `G x <= h` の単一変数行として畳み込まれて
//! いるため、まず [`extract_bounds`] で `lb`/`ub` と多変数行 (real rows) に
//! 分離し、`passes` 回まで伝播を反復する (Gauss-Seidel 型: 同じパス内で前の
//! 行が締めた境界を後の行が読む。行並列化すると収束挙動が変わるので逐次)。
//! 等式系 `A x = b` 側の伝播は [`propagate_equalities`] が行う。
//!
//! 開発経緯・並列化を見送った理由などは改良履歴メモを参照。

use crate::sparse::{FaerCsr, CsrRowBuilder, csr_from_rows, csr_row_iter, csr_row_vec};
use crate::params::presolve::{EQPROP_RELTOL, PROPAGATE_EPS, PROP_CANCEL_GUARD, PROP_RELTOL};
use super::infeas_tol;

/// 伝播のパスの打ち切り条件 ([`propagate_split`]・[`propagate_equalities`])。
///
/// 行も上下限も変わらなくなったパスではいつも止まる。そのほかに:
/// - `min_passes` パスまでは、変化があれば (どんなに小さくても) 続ける。
/// - それを超えたパスでは有意な境界の変化 (無限の境界が有限になる・有限の境界が相対 `sig_reltol` を超えて動く)
///   だけを適用し、直前のパスに有意な変化 (行の削除・強制行による固定を含む) があったときだけ続ける。
///   巡回的な行構造では上下限が等比級数的に少しずつ削られ続けて変化が 0 にならないので、有意な変化で
///   区切らないと不動点に達しない。小さな変化まで適用し続けると、上下限が少しずつ一点に寄って交差した
///   ところで固定され、丸め誤差の溜まった値で固定された列だけの等式行が `foldfixed` の許容誤差を超えて
///   偽の実行不能になる (bore3d、伝播 20 パス)。
/// - 作業量 (読んだ行の非零の延べ数) が `max_work` 以上になったパスの後、または `max_passes` パスで打ち切る。
#[derive(Clone, Copy, Debug)]
pub struct PassLimit {
    /// 小さな変化でも続けるパス数。
    pub min_passes: usize,
    /// パス数の上限。
    pub max_passes: usize,
    /// `min_passes` を超えたパスを続けるための、有限の境界の相対変化の閾値
    /// (`|new - old| > sig_reltol * (1 + |old|)`)。
    pub sig_reltol: f64,
    /// 作業量 (読んだ非零の延べ数) の上限。
    pub max_work: usize,
}

impl PassLimit {
    /// 従来の打ち切り: 変化がある限り最大 `passes` パス。
    pub fn fixed(passes: usize) -> Self {
        PassLimit { min_passes: passes, max_passes: passes, sig_reltol: 0.0, max_work: usize::MAX }
    }
}

impl From<usize> for PassLimit {
    fn from(passes: usize) -> Self {
        PassLimit::fixed(passes)
    }
}

/// 有限の境界 `old` から `new` への変化が `sig_reltol` の意味で有意か (無限の境界からの変化は常に有意)。
#[inline]
fn significant_change(old: f64, new: f64, sig_reltol: f64) -> bool {
    !old.is_finite() || (old - new).abs() > sig_reltol * (1.0 + old.abs())
}

/// [`propagate`] の結果 (境界を再度畳み込んだ `g`/`h` を含む完全版)。
///
/// パイプライン本体は [`PropagateSplit`] を使う。こちらはテストと
/// `G` 形式を必要とする呼び出し元向けに残している。
#[allow(dead_code)] // the pipeline uses `PropagateSplit`; kept for tests / G-form callers
pub struct PropagateResult {
    /// 伝播後の不等式行列 `G` (多変数行 + 各有限境界の単一変数行)。
    pub g: FaerCsr,
    /// `g` に対応する右辺 `h`。
    pub h: Vec<f64>,
    /// 伝播後の変数下限 (`g`/`h` の境界行から取り出したものと同一)。
    pub lb: Vec<f64>,
    /// 伝播後の変数上限。
    pub ub: Vec<f64>,
    /// 生き残った多変数行 (境界行は含まない)。`extract_bounds(n, &g, &h)` の
    /// 第 3 戻り値と同じもの。
    pub real_rows: Vec<Vec<(usize, f64)>>,
    /// `real_rows` の右辺。
    pub real_rhs: Vec<f64>,
    /// 実行不能を検出したら `true` (このとき他のフィールドは空)。
    pub infeasible: bool,
}

/// `G x <= h` を、単一変数の境界行 (→ `lb`/`ub`) と多変数行 (→ `rows`/`rhs`)
/// に分離する。
///
/// 境界行は位置ではなく行の形 (`len() == 1`) で判定するので、伝播前後や
/// `build_a_g` 直後のどの `G` にも使える。境界行がない変数は `-inf`/`+inf`。
/// 同じ変数に複数の境界行があれば最もきつい値を採る。
/// 戻り値は `(lb, ub, rows, rhs)`。
pub fn extract_bounds(n: usize, g: &FaerCsr, h: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<Vec<(usize, f64)>>, Vec<f64>) {
    let mut lb = vec![f64::NEG_INFINITY; n];
    let mut ub = vec![f64::INFINITY; n];
    let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut rhs: Vec<f64> = Vec::new();

    let gr = g.as_ref();
    for i in 0..gr.nrows() {
        // 境界行は CSR スライスから直接読む (所有コピーが要るのは多変数行だけ)。
        let cols = gr.col_indices_of_row_raw(i);
        if cols.len() == 1 {
            let (j, v) = (cols[0], gr.values_of_row(i)[0]);
            // v * x_j <= h_i を x_j の境界値に直したもの
            let bound = h[i] / v;
            if v > 0.0 {
                if bound < ub[j] {
                    ub[j] = bound;
                }
            } else if bound > lb[j] {
                lb[j] = bound;
            }
        } else {
            rows.push(csr_row_vec(g, i));
            rhs.push(h[i]);
        }
    }
    (lb, ub, rows, rhs)
}

/// [`extract_bounds`] の `lb`/`ub` だけを返す版 (値は同一、多変数行のコピーを省く)。
pub fn extract_bounds_only(n: usize, g: &FaerCsr, h: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut lb = vec![f64::NEG_INFINITY; n];
    let mut ub = vec![f64::INFINITY; n];
    let gr = g.as_ref();
    for i in 0..gr.nrows() {
        let cols = gr.col_indices_of_row_raw(i);
        if cols.len() == 1 {
            let (j, v) = (cols[0], gr.values_of_row(i)[0]);
            let bound = h[i] / v;
            if v > 0.0 {
                if bound < ub[j] {
                    ub[j] = bound;
                }
            } else if bound > lb[j] {
                lb[j] = bound;
            }
        }
    }
    (lb, ub)
}

/// ある変数で `lb > ub + tol` (境界が互いに矛盾) なら `true` ([`crossing_tol`] 参照)。
///
/// 単一変数制約と箱境界が矛盾する場合などの真の実行不能を検出する。
/// `lb`/`ub` を他で使う前 (特に `dualfix` が片側に固定して矛盾を消して
/// しまう前) に必ず確認すること。
pub fn bounds_inconsistent(n: usize, lb: &[f64], ub: &[f64]) -> bool {
    (0..n).any(|j| lb[j] > ub[j] + PROPAGATE_EPS && lb[j] > ub[j] + crossing_tol(lb[j], ub[j]))
}

/// 境界の交差 `lb > ub` を実行不能とみなす閾値。既定は `PROPAGATE_EPS * (1 + max(|lb|, |ub|))`
/// (有限な方だけ。相対形、作業 #9)、`ENOMOTO_T_PRESOLVE_REL_TOL=0` で従来の絶対 `PROPAGATE_EPS`。
#[inline]
fn crossing_tol(lb: f64, ub: f64) -> f64 {
    let scale = match (lb.is_finite(), ub.is_finite()) {
        (true, true) => lb.abs().max(ub.abs()),
        (true, false) => lb.abs(),
        (false, true) => ub.abs(),
        (false, false) => 0.0,
    };
    infeas_tol(PROPAGATE_EPS, scale)
}

/// 行の項 `(j, v)` から列 `k` を除いた最小活動度 (`upper == false`) または最大活動度 (`upper == true`) を
/// 直接足し合わせて求める (`k` 以外の読む境界はすべて有限であること)。
/// `finite_sum - contrib_k` の桁落ち (k の寄与が行を支配するとき) を避けるのに使う ([`PROP_CANCEL_GUARD`] 参照)。
#[inline]
fn activity_excluding(row: impl Iterator<Item = (usize, f64)>, k: usize, lb: &[f64], ub: &[f64], upper: bool) -> f64 {
    let mut s = 0.0f64;
    for (j, v) in row {
        if v == 0.0 || j == k {
            continue;
        }
        s += if (v > 0.0) != upper { v * lb[j] } else { v * ub[j] };
    }
    s
}

/// 行 (項数 `len`) の活動度 `finite_sum` から列 k (係数 `aik`) の寄与 `contrib` を除いた活動度 `l_s`。
///
/// 通常は引き算 `finite_sum - contrib` (従来と同じ式、同じビット)。ただし、その桁落ち誤差の見積もり
/// `len * eps * |contrib|` (finite_sum の部分和は |contrib| 級を通るので、以後の加算ごとに ulp(|contrib|) 程度の誤差が入る)
/// から来る候補 `(rhs - l_s) / aik` の誤差 `err / |aik|` が、後段の実行不能判定の許容誤差
/// `PROPAGATE_EPS * (1 + |候補|)` の `guard` 倍を超えうるときだけ、`recompute` (k を除いた直接和) に切り替える。
/// `guard == 0` で常に引き算。
#[inline]
fn excluded_activity(finite_sum: f64, contrib: f64, rhs: f64, aik: f64, len: usize, guard: f64, recompute: impl FnOnce() -> f64) -> f64 {
    let l_s = finite_sum - contrib;
    if guard > 0.0 {
        let err = len as f64 * f64::EPSILON * contrib.abs();
        if err > guard * PROPAGATE_EPS * (aik.abs() + (rhs - l_s).abs()) {
            return recompute();
        }
    }
    l_s
}

/// [`propagate`] の結果から再畳み込みした `g`/`h` を除いたもの。
///
/// プレソルブ・パイプライン内の呼び出し元 (`run_extended`, `dualpropagate`)
/// は分離形だけを読むので、CSR を組み立てない。
pub struct PropagateSplit {
    /// 伝播後の変数下限。
    pub lb: Vec<f64>,
    /// 伝播後の変数上限。
    pub ub: Vec<f64>,
    /// 生き残った多変数行。
    pub real_rows: Vec<Vec<(usize, f64)>>,
    /// `real_rows` の右辺。
    pub real_rhs: Vec<f64>,
    /// 実行不能を検出したら `true` (このとき他のフィールドは空)。
    pub infeasible: bool,
    /// 実行したパス数 (`ENOMOTO_DEBUG_PRESOLVE_ROUNDS` の表示用)。
    pub passes_used: usize,
    /// 行も上下限も変わらないパス、または有意な変化のないパスで止まったか
    /// (`false` = パス数か作業量の上限で打ち切り)。
    pub converged: bool,
    /// 作業量 (読んだ行の非零の延べ数)。
    pub work: usize,
}

impl PropagateSplit {
    /// 実行不能を表す空の結果を作る。
    fn infeasible() -> Self {
        PropagateSplit { lb: Vec::new(), ub: Vec::new(), real_rows: Vec::new(), real_rhs: Vec::new(), infeasible: true, passes_used: 0, converged: true, work: 0 }
    }
}

/// `G x <= h` に制約伝播を最大 `passes` パス適用し、`G`/`h` を再構築して返す。
///
/// `n` は変数数。内部は [`propagate_without_g_rebuild`] + [`rebuild_g_ref`]。
#[allow(dead_code)]
pub fn propagate(n: usize, g: &FaerCsr, h: &[f64], passes: usize) -> PropagateResult {
    // 分離形での伝播結果
    let split = propagate_without_g_rebuild(n, g, h, passes);
    if split.infeasible {
        return PropagateResult {
            g: csr_from_rows(&[], n),
            h: Vec::new(),
            lb: Vec::new(),
            ub: Vec::new(),
            real_rows: Vec::new(),
            real_rhs: Vec::new(),
            infeasible: true,
        };
    }
    let (new_g, new_h) = rebuild_g_ref(n, &split.real_rows, &split.real_rhs, &split.lb, &split.ub);
    PropagateResult { g: new_g, h: new_h, lb: split.lb, ub: split.ub, real_rows: split.real_rows, real_rhs: split.real_rhs, infeasible: false }
}

/// [`propagate`] の `g`/`h` 再構築を省いた版 (`lb`/`ub`/多変数行はビット単位で同一)。
pub fn propagate_without_g_rebuild(n: usize, g: &FaerCsr, h: &[f64], passes: impl Into<PassLimit>) -> PropagateSplit {
    let (lb, ub, rows, rhs) = extract_bounds(n, g, h);
    propagate_split(n, lb, ub, rows, rhs, passes)
}

/// 既に分離済みの `G` (`lb`/`ub` と多変数行 `rows`/`rhs`、[`extract_bounds`]
/// の戻り値と同じ形) に対して制約伝播を行う本体。
///
/// 各パスで各行について: 実行不能判定 → 冗長行削除 → 強制行の固定と削除 →
/// 境界強化 (ZIB Report 16-44 §3.2)。パスの打ち切りは [`PassLimit`] を参照
/// (`usize` を渡すと従来どおり、変化がある限り最大その数のパス)。
/// 最後に境界の自己矛盾を再確認する。
pub fn propagate_split(n: usize, mut lb: Vec<f64>, mut ub: Vec<f64>, mut rows: Vec<Vec<(usize, f64)>>, mut rhs: Vec<f64>, passes: impl Into<PassLimit>) -> PropagateSplit {
    let limit: PassLimit = passes.into();
    // 相対改善閾値 (`ENOMOTO_T_PROP_RELTOL`、0 で無効)。有限境界は改善幅が
    // `reltol * (1 + |bound|)` を超えるときだけ更新する (HiGHS 流、巡回構造での
    // 境界の幾何級数的な削り込みを止める)。
    let reltol = tunable!("ENOMOTO_T_PROP_RELTOL", PROP_RELTOL, f64);
    // 桁落ち回避の閾値 (`ENOMOTO_T_PROP_CANCEL_GUARD`、0 で無効)
    let cancel_guard = tunable!("ENOMOTO_T_PROP_CANCEL_GUARD", PROP_CANCEL_GUARD, f64);

    if bounds_inconsistent(n, &lb, &ub) {
        return PropagateSplit::infeasible();
    }

    let mut infeasible = false;
    let mut passes_used = 0usize;
    let mut converged = false;
    let mut work = 0usize;
    for _pass in 0..limit.max_passes {
        if infeasible {
            break;
        }
        passes_used += 1;
        // `min_passes` を超えたパスでは有意な境界の変化だけを適用する (`PassLimit` 参照)。
        let strict = passes_used > limit.min_passes;
        // このパスで行削除または境界更新があったか (なければ以降のパスも同じなので打ち切る)
        let mut changed = false;
        // このパスで有意な境界の変化があったか (`PassLimit` 参照。行の削除は下で別に数える)
        let mut significant = false;
        let n_rows_before = rows.len();
        let mut kept_rows = Vec::with_capacity(rows.len());
        let mut kept_rhs = Vec::with_capacity(rhs.len());
        for (row, b) in std::mem::take(&mut rows).into_iter().zip(std::mem::take(&mut rhs)) {
            work += row.len();
            // 有限な項だけの最小/最大活動度の和
            let mut finite_sum_inf = 0.0f64;
            let mut finite_sum_sup = 0.0f64;
            // finite_sum_inf の各項の絶対値の和 (実行不能判定の許容誤差と桁落ちの判定に使う)
            let mut finite_abs_inf = 0.0f64;
            // 最小活動度を -inf にする項の個数と、その最初の変数 (個数が 1 のときだけ使う)
            let mut inf_unbounded_count = 0usize;
            let mut inf_unbounded_first = usize::MAX;
            // 最大活動度を +inf にする項の個数
            let mut sup_unbounded_count = 0usize;

            for &(j, v) in &row {
                if v > 0.0 {
                    if lb[j].is_finite() {
                        finite_sum_inf += v * lb[j];
                        finite_abs_inf += (v * lb[j]).abs();
                    } else {
                        {
                            if inf_unbounded_count == 0 {
                                inf_unbounded_first = j;
                            }
                            inf_unbounded_count += 1;
                        }
                    }
                    if ub[j].is_finite() {
                        finite_sum_sup += v * ub[j];
                    } else {
                        sup_unbounded_count += 1;
                    }
                } else {
                    if ub[j].is_finite() {
                        finite_sum_inf += v * ub[j];
                        finite_abs_inf += (v * ub[j]).abs();
                    } else {
                        {
                            if inf_unbounded_count == 0 {
                                inf_unbounded_first = j;
                            }
                            inf_unbounded_count += 1;
                        }
                    }
                    if lb[j].is_finite() {
                        finite_sum_sup += v * lb[j];
                    } else {
                        sup_unbounded_count += 1;
                    }
                }
            }

            // 行の真の最小/最大活動度 (無限の項があれば ±inf)
            let min_activity = if inf_unbounded_count == 0 { finite_sum_inf } else { f64::NEG_INFINITY };
            let max_activity = if sup_unbounded_count == 0 { finite_sum_sup } else { f64::INFINITY };

            // 実行不能判定の許容誤差は既定で `PROPAGATE_EPS * (1 + max(Σ|項|, |b|))` (相対形、作業 #9)。
            // 絶対 `PROPAGATE_EPS` を超えるがこの範囲に収まる違反は、下の強制行として扱う。
            if min_activity > b + PROPAGATE_EPS && min_activity > b + infeas_tol(PROPAGATE_EPS, finite_abs_inf.max(b.abs())) {
                infeasible = true;
                break;
            }
            if max_activity <= b + PROPAGATE_EPS {
                // 決して破られない行: 冗長なので削除 (ZIB Report 16-44 §3.1)。
                continue;
            }

            // 強制行 (HiGHS の rowPresolve と同じ): 最小活動度 (有限) が b に等しければ、
            // 全項を最小活動度を与える側の境界 (正係数は下限、負係数は上限) に固定でき、
            // 行自体は自明に満たされるので削除する。`inf_unbounded_count == 0` により
            // 読む境界はすべて有限。上の判定を通っているので `finite_sum_inf > b + PROPAGATE_EPS` は
            // 許容誤差内の違反 (相対形のときだけ起こる) で、これも強制行とみなす (従来の `|finite_sum_inf - b| <= EPS` と、
            // 絶対形のときは同値)。
            if inf_unbounded_count == 0 && finite_sum_inf >= b - PROPAGATE_EPS {
                for &(j, v) in &row {
                    if v > 0.0 {
                        ub[j] = lb[j];
                    } else {
                        lb[j] = ub[j];
                    }
                }
                continue;
            }

            // 境界強化 (ZIB Report 16-44 §3.2): l_s は x_k の寄与を除いた最小活動度。x_k 以外に
            // 無限の寄与がない場合だけ有限になる。
            for &(k, aik) in &row {
                let l_s = if inf_unbounded_count == 0 {
                    let contrib_k = if aik > 0.0 { aik * lb[k] } else { aik * ub[k] };
                    excluded_activity(finite_sum_inf, contrib_k, b, aik, row.len(), cancel_guard, || activity_excluding(row.iter().copied(), k, &lb, &ub, false))
                } else if inf_unbounded_count == 1 && inf_unbounded_first == k {
                    finite_sum_inf
                } else {
                    continue;
                };
                if !l_s.is_finite() {
                    continue;
                }
                if aik > 0.0 {
                    // 新しい上限候補
                    let candidate = (b - l_s) / aik;
                    if candidate < ub[k] - PROPAGATE_EPS && (reltol == 0.0 || !ub[k].is_finite() || candidate < ub[k] - reltol * (1.0 + ub[k].abs())) && (!strict || significant_change(ub[k], candidate, limit.sig_reltol)) {
                        significant |= significant_change(ub[k], candidate, limit.sig_reltol);
                        ub[k] = candidate;
                        changed = true;
                    }
                } else if aik < 0.0 {
                    // 新しい下限候補
                    let candidate = (b - l_s) / aik;
                    if candidate > lb[k] + PROPAGATE_EPS && (reltol == 0.0 || !lb[k].is_finite() || candidate > lb[k] + reltol * (1.0 + lb[k].abs())) && (!strict || significant_change(lb[k], candidate, limit.sig_reltol)) {
                        significant |= significant_change(lb[k], candidate, limit.sig_reltol);
                        lb[k] = candidate;
                        changed = true;
                    }
                }
            }

            kept_rows.push(row);
            kept_rhs.push(b);
        }

        if kept_rows.len() != n_rows_before {
            changed = true;
            significant = true;
        }
        rows = kept_rows;
        rhs = kept_rhs;
        if !changed || (passes_used >= limit.min_passes && !significant) {
            converged = true;
            break;
        }
        if work >= limit.max_work {
            break;
        }
    }

    // 行ごとの判定では検出されなくても、別々の行が lb/ub を互いに逆側へ
    // 押して交差させることがあるので、全パス後にもう一度確認する。
    if !infeasible && bounds_inconsistent(n, &lb, &ub) {
        infeasible = true;
    }
    if !infeasible {
        // 許容誤差内の交差のうち、従来の絶対 `PROPAGATE_EPS` を超えるもの (相対形のときだけ残る) は 1 点に揃える
        // (`propagate_equalities` と同じ)。`PROPAGATE_EPS` 以内の交差は従来どおり残す (出力を変えないため)。
        for j in 0..n {
            if lb[j] > ub[j] + PROPAGATE_EPS {
                lb[j] = ub[j];
            }
        }
    }

    if infeasible {
        return PropagateSplit::infeasible();
    }

    PropagateSplit { lb, ub, real_rows: rows, real_rhs: rhs, infeasible: false, passes_used, converged, work }
}

/// 各パスが読む `G x <= h` の表現。CSR 実体か、分離形 `(rows, rhs, lb, ub)` のどちらか。
///
/// 分離形は `rebuild_g_ref(rows, rhs, lb, ub)` と厳密に同じ行列を表し、
/// [`split_is_canonical`] が成り立つとき (= `extract_bounds` で元の分離形が
/// ビット単位で戻るとき) だけ使う。CSR を組まずに済ませるための仕組み。
#[derive(Clone, Copy)]
pub enum GView<'a> {
    /// CSR 実体 `g` と右辺 `h`。
    Mat { g: &'a FaerCsr, h: &'a [f64] },
    /// 分離形: 多変数行 `rows`/`rhs` と境界 `lb`/`ub`。
    Split { rows: &'a [Vec<(usize, f64)>], rhs: &'a [f64], lb: &'a [f64], ub: &'a [f64] },
}

impl GView<'_> {
    /// このビューが表す行列の行数 (分離形では多変数行 + 有限境界の数)。
    pub fn nrows(&self) -> usize {
        match *self {
            GView::Mat { g, .. } => g.nrows(),
            GView::Split { rows, lb, ub, .. } => rows.len() + lb.iter().filter(|v| v.is_finite()).count() + ub.iter().filter(|v| v.is_finite()).count(),
        }
    }

    /// このビューが表す行列に対する `extract_bounds_only` の結果 `(lb, ub)`。
    /// 分離形なら借用、CSR なら新規計算。
    pub fn bounds(&self, n: usize) -> (std::borrow::Cow<'_, [f64]>, std::borrow::Cow<'_, [f64]>) {
        match *self {
            GView::Mat { g, h } => {
                let (lb, ub) = extract_bounds_only(n, g, h);
                (std::borrow::Cow::Owned(lb), std::borrow::Cow::Owned(ub))
            }
            GView::Split { lb, ub, .. } => (std::borrow::Cow::Borrowed(lb), std::borrow::Cow::Borrowed(ub)),
        }
    }
}

/// `rebuild_g_ref(rows, _, lb, ub)` が [`extract_bounds`] を通して厳密に往復するか。
///
/// 条件: 全行が 2 項以上・格納ゼロなし・列番号が範囲内で狭義単調増加
/// (そのまま格納され境界行と誤認されない)、かつ下限は有限か `-inf`、
/// 上限は有限か `+inf` (逆側の無限大や NaN は往復で別の値に化ける)。
pub fn split_is_canonical(n: usize, rows: &[Vec<(usize, f64)>], lb: &[f64], ub: &[f64]) -> bool {
    lb.iter().all(|&v| v.is_finite() || v == f64::NEG_INFINITY)
        && ub.iter().all(|&v| v.is_finite() || v == f64::INFINITY)
        && rows.iter().all(|r| r.len() >= 2 && r.iter().all(|&(j, v)| v != 0.0 && j < n) && r.windows(2).all(|w| w[0].0 < w[1].0))
}

/// 多変数行 + 有限境界ごとの単一変数行から `G x <= h` を再構築する
/// ([`extract_bounds`] の逆)。[`rebuild_g`] の借用版。
///
/// 行を所有・複製せず、境界行ごとの `Vec` も作らずに CSR を直接組む。
/// 結果は [`rebuild_g`] とビット単位で同一。多変数行に重複列があって
/// ビルダーが拒否した場合は [`rebuild_g`] にフォールバックする。
/// 行の並びは `rows`、続いて変数順に (上限行 `x_j <= ub_j`, 下限行 `-x_j <= -lb_j`)。
pub fn rebuild_g_ref(n: usize, rows: &[Vec<(usize, f64)>], rhs: &[f64], lb: &[f64], ub: &[f64]) -> (FaerCsr, Vec<f64>) {
    // 有限境界の個数 = 追加する境界行の数
    let n_bounds = (0..n).filter(|&j| ub[j].is_finite()).count() + (0..n).filter(|&j| lb[j].is_finite()).count();
    let nnz: usize = rows.iter().map(|r| r.len()).sum::<usize>() + n_bounds;
    let mut builder = CsrRowBuilder::with_capacity(n, rows.len() + n_bounds, nnz);
    for row in rows {
        if !builder.push_row(row) {
            return rebuild_g(n, rows.to_vec(), rhs.to_vec(), lb, ub);
        }
    }
    let mut new_rhs = Vec::with_capacity(rhs.len() + n_bounds);
    new_rhs.extend_from_slice(rhs);
    for j in 0..n {
        if ub[j].is_finite() {
            builder.push_singleton(j, 1.0);
            new_rhs.push(ub[j]);
        }
        if lb[j].is_finite() {
            builder.push_singleton(j, -1.0);
            new_rhs.push(-lb[j]);
        }
    }
    (builder.finish(), new_rhs)
}

/// 多変数行 `rows`/`rhs` の末尾に有限境界ごとの単一変数行を追加して
/// `G x <= h` を再構築する ([`extract_bounds`] の逆)。`dualfix` 等も使う。
pub fn rebuild_g(n: usize, mut rows: Vec<Vec<(usize, f64)>>, mut rhs: Vec<f64>, lb: &[f64], ub: &[f64]) -> (FaerCsr, Vec<f64>) {
    for j in 0..n {
        if ub[j].is_finite() {
            rows.push(vec![(j, 1.0)]);
            rhs.push(ub[j]);
        }
        if lb[j].is_finite() {
            rows.push(vec![(j, -1.0)]);
            rhs.push(-lb[j]);
        }
    }
    (csr_from_rows(&rows, n), rhs)
}

/// 不等式伝播 ([`propagate`]) のテスト。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    /// 最小活動度が右辺に一致する強制行で、全変数が下限に固定され行が削除されることを確認。
    #[test]
    fn forcing_row_fixes_every_variable_to_its_minimum_bound() {
        // x0 in [1,3], x1 in [2,4], 行 x0 + x1 <= 3。最小活動度 1+2=3 が右辺と一致。
        let g = csr_from_rows(
            &[
                vec![(0, 1.0)],
                vec![(0, -1.0)],
                vec![(1, 1.0)],
                vec![(1, -1.0)],
                vec![(0, 1.0), (1, 1.0)],
            ],
            2,
        );
        let h = vec![3.0, -1.0, 4.0, -2.0, 3.0];
        let result = propagate(2, &g, &h, 1);
        assert!(!result.infeasible);
        assert!((result.lb[0] - 1.0).abs() < 1e-9, "lb={:?}", result.lb);
        assert!((result.ub[0] - 1.0).abs() < 1e-9, "ub={:?}", result.ub);
        assert!((result.lb[1] - 2.0).abs() < 1e-9, "lb={:?}", result.lb);
        assert!((result.ub[1] - 2.0).abs() < 1e-9, "ub={:?}", result.ub);
        // 強制行自体は多変数行から消える
        assert!(result.real_rows.is_empty(), "real_rows={:?}", result.real_rows);
    }

    /// 係数の符号が混在する強制行で、項ごとに対応する側の境界に固定されることを確認。
    #[test]
    fn forcing_row_with_mixed_signs_uses_the_matching_bound_per_term() {
        // x0 in [2,10], x1 in [0,6], 行 x0 - x1 <= -4。最小活動度 2 - 6 = -4 = b。
        // x0=2 (正係数→下限)、x1=6 (負係数→上限) に固定される。
        let g = csr_from_rows(
            &[
                vec![(0, 1.0)],
                vec![(0, -1.0)],
                vec![(1, 1.0)],
                vec![(1, -1.0)],
                vec![(0, 1.0), (1, -1.0)],
            ],
            2,
        );
        let h = vec![10.0, -2.0, 6.0, 0.0, -4.0];
        let result = propagate(2, &g, &h, 1);
        assert!(!result.infeasible);
        assert!((result.lb[0] - 2.0).abs() < 1e-9, "lb={:?}", result.lb);
        assert!((result.ub[0] - 2.0).abs() < 1e-9, "ub={:?}", result.ub);
        assert!((result.lb[1] - 6.0).abs() < 1e-9, "lb={:?}", result.lb);
        assert!((result.ub[1] - 6.0).abs() < 1e-9, "ub={:?}", result.ub);
        assert!(result.real_rows.is_empty(), "real_rows={:?}", result.real_rows);
    }
}

/// [`propagate_equalities`] の結果と統計。
pub struct EqPropagateResult {
    /// 等式行が満たせないことを検出したら `true`。
    pub infeasible: bool,
    /// 強制行として検出された等式行の数 (同じ行は 1 回だけ数える)。
    pub forcing_rows: usize,
    /// 強制行によって固定された (それまで lb < ub だった) 変数の延べ数。
    pub fixed_cols: usize,
    /// 境界強化で更新された境界の延べ数。
    pub tightened: usize,
    /// 変化のないパス、または有意な変化のないパスで止まったか (`false` = パス数か作業量の上限で打ち切り)。
    pub converged: bool,
    /// 実行したパス数。
    pub passes_used: usize,
    /// 作業量 (読んだ行の非零の延べ数)。
    pub work: usize,
}

/// 等式系 `A x = b` に対する活動度ベースの伝播 ([`propagate`] は不等式系しか見ない)。
///
/// 各等式行を `A_i x <= b_i` と `A_i x >= b_i` の 2 本とみなし、両側の強制行
/// 検出 (全項が一方の境界に張り付くときだけ b に届く) と、有限な側からの境界
/// 強化を行う。書き換えるのは `lb`/`ub` だけで、行自体の縮約は `foldfixed` や
/// rowsingleton/doubleton/colsingleton に任せる。等式にしか現れない無限境界の
/// 列に有限境界を与え、傾き・切片二段解法の M 側処理を減らすのが目的。
/// パスの打ち切りは [`PassLimit`] を参照 (`usize` なら従来どおり最大その数のパス、変化がなければ打ち切り)。
pub fn propagate_equalities(a: &FaerCsr, b: &[f64], lb: &mut [f64], ub: &mut [f64], passes: impl Into<PassLimit>) -> EqPropagateResult {
    let limit: PassLimit = passes.into();
    let ar = a.as_ref();
    // 相対改善閾値 (`ENOMOTO_T_EQPROP_RELTOL`、0 で無効)。
    let reltol = tunable!("ENOMOTO_T_EQPROP_RELTOL", EQPROP_RELTOL, f64);
    // 桁落ち回避の閾値 (`ENOMOTO_T_PROP_CANCEL_GUARD`、0 で無効)
    let cancel_guard = tunable!("ENOMOTO_T_PROP_CANCEL_GUARD", PROP_CANCEL_GUARD, f64);
    // 境界 old を new に置き換えるべきか: 無限の境界は常に置き換え、有限なら
    // 変化量が PROPAGATE_EPS (と reltol * (1 + |old|)) を超えるときだけ。
    let improves = |old: f64, new: f64| -> bool {
        if !old.is_finite() {
            return true;
        }
        (old - new).abs() > PROPAGATE_EPS && (reltol == 0.0 || (old - new).abs() > reltol * (1.0 + old.abs()))
    };
    let mut res = EqPropagateResult { infeasible: false, forcing_rows: 0, fixed_cols: 0, tightened: 0, converged: false, passes_used: 0, work: 0 };
    // 行 i を既に強制行として数えたか (forcing_rows の重複カウント防止)
    let mut forcing_seen = vec![false; ar.nrows()];
    for _pass in 0..limit.max_passes {
        res.passes_used += 1;
        // `min_passes` を超えたパスでは有意な境界の変化だけを適用する (`PassLimit` 参照)。
        let strict = res.passes_used > limit.min_passes;
        // このパスでの変更回数
        let mut changed = 0usize;
        // このパスで有意な変化 (`PassLimit` 参照) があったか
        let mut significant = false;
        for i in 0..ar.nrows() {
            let bi = b[i];
            let row_len = ar.col_indices_of_row_raw(i).len();
            res.work += row_len;
            // 有限な項だけの最小/最大活動度の和
            let mut finite_sum_inf = 0.0f64;
            let mut finite_sum_sup = 0.0f64;
            // それぞれの項の絶対値の和 (実行不能判定の許容誤差と桁落ちの判定に使う)
            let mut finite_abs_inf = 0.0f64;
            let mut finite_abs_sup = 0.0f64;
            // 最小/最大活動度を無限にする変数の一覧
            let mut inf_unbounded: Vec<usize> = Vec::new();
            let mut sup_unbounded: Vec<usize> = Vec::new();
            // まだ固定されていない (lb < ub) 変数の数
            let mut live = 0usize;
            for (j, v) in csr_row_iter(a, i) {
                if v == 0.0 {
                    continue;
                }
                if lb[j] < ub[j] {
                    live += 1;
                }
                if v > 0.0 {
                    if lb[j].is_finite() {
                        finite_sum_inf += v * lb[j];
                        finite_abs_inf += (v * lb[j]).abs();
                    } else {
                        inf_unbounded.push(j);
                    }
                    if ub[j].is_finite() {
                        finite_sum_sup += v * ub[j];
                        finite_abs_sup += (v * ub[j]).abs();
                    } else {
                        sup_unbounded.push(j);
                    }
                } else {
                    if ub[j].is_finite() {
                        finite_sum_inf += v * ub[j];
                        finite_abs_inf += (v * ub[j]).abs();
                    } else {
                        inf_unbounded.push(j);
                    }
                    if lb[j].is_finite() {
                        finite_sum_sup += v * lb[j];
                        finite_abs_sup += (v * lb[j]).abs();
                    } else {
                        sup_unbounded.push(j);
                    }
                }
            }
            if live == 0 {
                continue;
            }
            // 行の真の最小/最大活動度
            let min_activity = if inf_unbounded.is_empty() { finite_sum_inf } else { f64::NEG_INFINITY };
            let max_activity = if sup_unbounded.is_empty() { finite_sum_sup } else { f64::INFINITY };
            // 実行不能判定の許容誤差は既定で `PROPAGATE_EPS * (1 + max(Σ|項|, |b|))` (相対形、作業 #9)。
            // 絶対 `PROPAGATE_EPS` を超えるがこの範囲に収まる違反は、下の強制行として扱う。
            if (min_activity > bi + PROPAGATE_EPS && min_activity > bi + infeas_tol(PROPAGATE_EPS, finite_abs_inf.max(bi.abs())))
                || (max_activity < bi - PROPAGATE_EPS && max_activity < bi - infeas_tol(PROPAGATE_EPS, finite_abs_sup.max(bi.abs())))
            {
                res.infeasible = true;
                return res;
            }
            // 下側の強制: 全項が最小活動度側の境界にあるときだけ b に届く。上の判定を通っているので
            // `finite_sum_inf > bi + PROPAGATE_EPS` は許容誤差内の違反 (相対形のときだけ起こる) で、これも強制とみなす
            // (絶対形のときは従来の `|finite_sum_inf - bi| <= EPS` と同値)。上側も対称。
            if inf_unbounded.is_empty() && finite_sum_inf >= bi - PROPAGATE_EPS {
                if !forcing_seen[i] {
                    forcing_seen[i] = true;
                    res.forcing_rows += 1;
                }
                for (j, v) in csr_row_iter(a, i) {
                    if v == 0.0 {
                        continue;
                    }
                    if lb[j] < ub[j] {
                        res.fixed_cols += 1;
                        changed += 1;
                        significant = true;
                    }
                    if v > 0.0 {
                        ub[j] = lb[j];
                    } else {
                        lb[j] = ub[j];
                    }
                }
                continue;
            }
            // 上側の強制: 対称に、全項が最大活動度側の境界。
            if sup_unbounded.is_empty() && finite_sum_sup <= bi + PROPAGATE_EPS {
                if !forcing_seen[i] {
                    forcing_seen[i] = true;
                    res.forcing_rows += 1;
                }
                for (j, v) in csr_row_iter(a, i) {
                    if v == 0.0 {
                        continue;
                    }
                    if lb[j] < ub[j] {
                        res.fixed_cols += 1;
                        changed += 1;
                        significant = true;
                    }
                    if v > 0.0 {
                        lb[j] = ub[j];
                    } else {
                        ub[j] = lb[j];
                    }
                }
                continue;
            }
            for (k, aik) in csr_row_iter(a, i) {
                if aik == 0.0 || lb[k] == ub[k] {
                    continue;
                }
                // `A_i x <= b` 側: k を除いた最小活動度 l_s から境界を得る。
                // k の寄与が行を支配するときは、引き算の桁落ちを避けて k を除いた和を直接計算する
                // (`PROP_CANCEL_GUARD` 参照。ken-18 の行 3749 で下限が相対 4e-14 ずれたのと同じ機序)。
                let l_s = if inf_unbounded.is_empty() {
                    let contrib = if aik > 0.0 { aik * lb[k] } else { aik * ub[k] };
                    Some(excluded_activity(finite_sum_inf, contrib, bi, aik, row_len, cancel_guard, || activity_excluding(csr_row_iter(a, i), k, lb, ub, false)))
                } else if inf_unbounded.len() == 1 && inf_unbounded[0] == k {
                    Some(finite_sum_inf)
                } else {
                    None
                };
                if let Some(l_s) = l_s {
                    let candidate = (bi - l_s) / aik;
                    if aik > 0.0 {
                        if candidate < ub[k] - PROPAGATE_EPS && improves(ub[k], candidate) && (!strict || significant_change(ub[k], candidate, limit.sig_reltol)) {
                            significant |= significant_change(ub[k], candidate, limit.sig_reltol);
                            ub[k] = candidate;
                            changed += 1;
                            res.tightened += 1;
                        }
                    } else if candidate > lb[k] + PROPAGATE_EPS && improves(lb[k], candidate) && (!strict || significant_change(lb[k], candidate, limit.sig_reltol)) {
                        significant |= significant_change(lb[k], candidate, limit.sig_reltol);
                        lb[k] = candidate;
                        changed += 1;
                        res.tightened += 1;
                    }
                }
                // `A_i x >= b` 側: k を除いた最大活動度 u_s から境界を得る。
                let u_s = if sup_unbounded.is_empty() {
                    let contrib = if aik > 0.0 { aik * ub[k] } else { aik * lb[k] };
                    Some(excluded_activity(finite_sum_sup, contrib, bi, aik, row_len, cancel_guard, || activity_excluding(csr_row_iter(a, i), k, lb, ub, true)))
                } else if sup_unbounded.len() == 1 && sup_unbounded[0] == k {
                    Some(finite_sum_sup)
                } else {
                    None
                };
                if let Some(u_s) = u_s {
                    let candidate = (bi - u_s) / aik;
                    if aik > 0.0 {
                        if candidate > lb[k] + PROPAGATE_EPS && improves(lb[k], candidate) && (!strict || significant_change(lb[k], candidate, limit.sig_reltol)) {
                            significant |= significant_change(lb[k], candidate, limit.sig_reltol);
                            lb[k] = candidate;
                            changed += 1;
                            res.tightened += 1;
                        }
                    } else if candidate < ub[k] - PROPAGATE_EPS && improves(ub[k], candidate) && (!strict || significant_change(ub[k], candidate, limit.sig_reltol)) {
                        significant |= significant_change(ub[k], candidate, limit.sig_reltol);
                        ub[k] = candidate;
                        changed += 1;
                        res.tightened += 1;
                    }
                }
                if lb[k] > ub[k] + PROPAGATE_EPS && lb[k] > ub[k] + crossing_tol(lb[k], ub[k]) {
                    res.infeasible = true;
                    return res;
                }
                if lb[k] > ub[k] {
                    // 許容誤差 (`crossing_tol`、既定は相対形) 以内の交差: 1 点に揃える。
                    lb[k] = ub[k];
                }
            }
        }
        if changed == 0 || (res.passes_used >= limit.min_passes && !significant) {
            res.converged = true;
            break;
        }
        if res.work >= limit.max_work {
            break;
        }
    }
    res
}

/// 等式伝播 ([`propagate_equalities`]) のテスト。
#[cfg(test)]
mod eqprop_tests {
    use super::*;

    /// 等式の強制行で各項が対応する側の境界に固定され、統計が正しく数えられることを確認。
    #[test]
    fn forcing_row_fixes_every_term_at_its_matching_bound() {
        // x0 in [2,10], x1 in [0,6], 行 x0 - x1 = -4。最小活動度 2 - 6 = -4 = b。
        let a = csr_from_rows(&[vec![(0, 1.0), (1, -1.0)]], 2);
        let b = vec![-4.0];
        let mut lb = vec![2.0, 0.0];
        let mut ub = vec![10.0, 6.0];
        let res = propagate_equalities(&a, &b, &mut lb, &mut ub, 1);
        assert!(!res.infeasible);
        assert_eq!(res.forcing_rows, 1);
        assert_eq!(res.fixed_cols, 2);
        assert!((lb[0] - 2.0).abs() < 1e-9 && (ub[0] - 2.0).abs() < 1e-9, "x0={:?}/{:?}", lb[0], ub[0]);
        assert!((lb[1] - 6.0).abs() < 1e-9 && (ub[1] - 6.0).abs() < 1e-9, "x1={:?}/{:?}", lb[1], ub[1]);
    }

    /// 自由変数が等式行から有限の境界を得ることを確認。
    #[test]
    fn free_column_gets_a_finite_bound_from_an_equality_row() {
        // x0 自由, x1 in [0,5], 行 x0 + x1 = 3 → x0 = 3 - x1 in [-2, 3]。
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let b = vec![3.0];
        let mut lb = vec![f64::NEG_INFINITY, 0.0];
        let mut ub = vec![f64::INFINITY, 5.0];
        let res = propagate_equalities(&a, &b, &mut lb, &mut ub, 2);
        assert!(!res.infeasible);
        assert!(res.tightened >= 2, "tightened={}", res.tightened);
        assert!((lb[0] - (-2.0)).abs() < 1e-9, "lb[0]={}", lb[0]);
        assert!((ub[0] - 3.0).abs() < 1e-9, "ub[0]={}", ub[0]);
    }

    /// 最大活動度が右辺に届かない等式行が実行不能と判定されることを確認。
    #[test]
    fn contradictory_equality_row_is_infeasible() {
        // x0, x1 in [0,1], 行 x0 + x1 = 5: 最大活動度 2 < 5。
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let b = vec![5.0];
        let mut lb = vec![0.0, 0.0];
        let mut ub = vec![1.0, 1.0];
        let res = propagate_equalities(&a, &b, &mut lb, &mut ub, 1);
        assert!(res.infeasible);
    }

    /// 桁落ちの安全網: 行 `a0 x0 + x1 + x2 + x3 = 0` で x0 の寄与 (a0 * ub0 = -1e12) が他の項 (~2) を 1e12 倍支配する。
    /// `finite_sum - contrib` の引き算では x0 の下限が相対 1e-5 級ずれ (相対 1e-7 のクランプでも吸収できず、
    /// 後段の rowsingleton が実行可能な問題を実行不能と誤判定しうる)、k を除いた和を直接計算すると
    /// 後段が計算する値 `-(Σ x_j) / a0` とビット単位で一致する。ken-18 (支配 650 倍、誤差 4e-14) の極端な版。
    #[test]
    fn dominant_term_bound_avoids_cancellation() {
        let a0 = -1e-3;
        let ub0 = 1e15;
        let lows = [0.3, 0.7, 1.1];
        let a = csr_from_rows(&[vec![(0, a0), (1, 1.0), (2, 1.0), (3, 1.0)]], 4);
        let b = vec![0.0];
        let mut lb = vec![0.0, lows[0], lows[1], lows[2]];
        let mut ub = vec![ub0, 10.0, 10.0, 10.0];
        let res = propagate_equalities(&a, &b, &mut lb, &mut ub, 1);
        assert!(!res.infeasible);
        let exact = -(lows[0] + lows[1] + lows[2]) / a0;
        // 旧式 (引き算) の値は相対 1e-7 を超えてずれる (この試験が桁落ちを実際に踏んでいることの確認)
        let finite_sum = a0 * ub0 + lows[0] + lows[1] + lows[2];
        let naive = -(finite_sum - a0 * ub0) / a0;
        assert!((naive - exact).abs() > 1e-7 * exact.abs(), "naive={naive} exact={exact}");
        assert_eq!(lb[0], exact);
    }

    /// 相対形の実行不能判定: スケール後の大きな量 (1e8) で、活動度が右辺を丸め程度 (相対 1e-15) だけ
    /// 超える等式行は実行不能ではなく強制行として扱う。相対 1e-6 超えるものは従来どおり実行不能。
    #[test]
    fn relative_tolerance_in_equality_infeasibility() {
        // x0 in [1e8, 2e8], x1 in [0, 1]: 行 x0 + x1 = 1e8 - 1e-7 (最小活動度が 1e-7 = 相対 1e-15 だけ超える)
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let b = vec![1e8 - 1e-7];
        let mut lb = vec![1e8, 0.0];
        let mut ub = vec![2e8, 1.0];
        let res = propagate_equalities(&a, &b, &mut lb, &mut ub, 1);
        assert!(!res.infeasible);
        assert_eq!((lb[0], ub[0], lb[1], ub[1]), (1e8, 1e8, 0.0, 0.0));
        // 相対 1e-6 のはみ出しは実行不能
        let b = vec![1e8 * (1.0 - 1e-6)];
        let mut lb = vec![1e8, 0.0];
        let mut ub = vec![2e8, 1.0];
        let res = propagate_equalities(&a, &b, &mut lb, &mut ub, 1);
        assert!(res.infeasible);
    }
}
