//! 整数変数を含むモデルのための分枝限定法 (計画は `docs/branch_and_cut_plan.md`)。
//!
//! - [`lp`]・[`lp_api`]: LP の基底・結果の型と、分枝限定法が LP に求める操作 (実装は `simplex::mip_lp::TwoStageLp`)。
//! - [`problem`]: 最小化形の問題データ。
//! - [`domain`]: 変数の定義域、変更の記録と巻き戻し、制約伝播。
//! - [`queue`]: 未処理ノードの待ち行列。
//! - [`pseudocost`]: pseudocost と分枝スコア。
//! - [`solver`]: 探索の本体 (reliability 分岐、plunge、暫定解による枝刈り)。

/// 分枝切除法の LP とやり取りする型 (基底・結果・打ち切り条件)。
pub(crate) mod lp;
pub(crate) mod idxset;
/// `ENOMOTO_MIP_FOLD` (既定): 色の細分化で MIP を畳む。
pub(crate) mod fold;
/// 完全単模の判定 (fold と implint_tu で共通)。
pub(crate) mod tu;
pub(crate) mod network;
pub(crate) mod tubr;
pub(crate) mod reflsym;
pub(crate) mod lp_api;
pub(crate) mod problem;
pub(crate) mod domain;
pub(crate) mod conflict_pool;
pub(crate) mod cutconf;
pub(crate) mod queue;
/// 試作 `ENOMOTO_MIP_CWM`: 衝突の重みの倍率を部分木ごとに割り振って対で比べる。
pub(crate) mod cwm;
pub(crate) mod replay;
pub(crate) mod pseudocost;
pub(crate) mod solver;
pub(crate) mod fj;
pub(crate) mod localmip;
pub(crate) mod fjsub;
mod heuristics;
mod t1primal;
mod subb;
mod alns_cp;
pub(crate) mod sympre;
mod sympre0;
pub(crate) mod heur_subfix;
pub(crate) mod condrev;
pub(crate) mod heur_scip;
pub(crate) mod rootlp3;
pub(crate) mod xolazy;
pub(crate) mod heur_ipm;
pub(crate) mod heur_shift;
pub(crate) mod symmetry;
pub(crate) mod symdyn;
pub(crate) mod cuts;
pub(crate) mod separation;
pub(crate) mod snfmodk;
pub(crate) mod zerohalf;
pub(crate) mod hnf;
pub(crate) mod hnflat;
pub(crate) mod hnfcut;
pub(crate) mod hnfsubst;
pub(crate) mod implint_tu;
pub(crate) mod cmirdelta;
pub(crate) mod seplearn;
pub(crate) mod sepalloc;
pub(crate) mod treedepth;
pub(crate) mod adapt;
pub(crate) mod cutmix;
pub(crate) mod mcf;
pub(crate) mod clique;
pub(crate) mod vbclq;
pub(crate) mod pretime;
pub(crate) mod tpre;
pub(crate) mod plsb;
pub(crate) mod prechain;
pub(crate) mod rootstop;
pub(crate) mod rcstrong;
pub(crate) mod fixmore;
pub(crate) mod objagg;
pub(crate) mod propcut;
pub(crate) mod lpbox;

use crate::solver::solve_lp;
use crate::types::{ConstraintRow, LpOptions, MipSettings, MipSummary, Objective, RootSolver, SolveResult, Status, VarType, VariableData};
use problem::MipProblem;
use solver::{MipParams, MipStatus};

/// 時間で決める上限 (根のカットのループ・探り) に使う時計 [秒]: このスレッドが使った CPU 時間 (Linux)。ほかの
/// プロセスに CPU を取られている間は進まないので、マシンの負荷で打ち切りの位置が変わりにくい (nw04: 負荷で根のカットの
/// 1 ラウンドが 1.9 -> 4.0 秒になり、時間の上限で 2 ラウンドで止まって根で閉じず 9 -> 17 秒)。Linux 以外と
/// `ENOMOTO_MIP_WALL_CAPS` では経過時間。差だけを使う。
/// `ENOMOTO_MIP_GR_AUTO` (試作、inv/gfrsplit-20261021): 一般整数 (0-1 でない整数) の列がある問題だけで
/// 一般整数の列への含意の implied bound カット (全体で使う試作 `ENOMOTO_MIP_IMPLBD_GENINT` は 2026-11-18 に削除) と `ENOMOTO_T_MIP_CUT_LP_RETRY=4` を働かせる。一般整数がなければ G はカットを増やさない
/// (プロービングで 0-1 の相手の含意を集めなくなるだけ) が、R はカットを入れた LP の時間の上限を延ばして根の経路を変える
/// (neos-960392 (全部 0-1) では根の RENS が解を出すかが根のカットの経路で決まり、gfr では 4 回とも -2 だった)。
/// `=2` なら一般整数がある問題では根の面の頂点の分離も止める (`ENOMOTO_MIP_FACE_SEP=off` と同じ。mzzv42z・roll3000 は
/// G+R に F がないと根が崩れる)
/// 2026-10-21 から既定で `=2` 相当 (`ENOMOTO_T_MIP_GR_AUTO_MIN_FRAC` 0.01 と合わせて、計測 QP・QC の 4 組で解けた数 +7、
/// SGM は 8 つ中 6 つ良い)。`ENOMOTO_MIP_NO_GR_AUTO` で使わない
pub(crate) fn gr_auto() -> bool {
    env_str!("ENOMOTO_MIP_GR_AUTO").is_some() || env_str!("ENOMOTO_MIP_NO_GR_AUTO").is_none()
}

/// [`gr_auto`] で根の面の頂点の分離も止めるか (`ENOMOTO_MIP_GR_AUTO=2`、既定。`ENOMOTO_MIP_GR_AUTO` に 2 以外を与えると止めない)
pub(crate) fn gr_auto_face_off() -> bool {
    match env_str!("ENOMOTO_MIP_GR_AUTO") {
        Some(v) => v == "2",
        None => env_str!("ENOMOTO_MIP_NO_GR_AUTO").is_none(),
    }
}

/// この問題で G (一般整数の含意の implied bound カット) を働かせるか ([`gr_auto`] で一般整数があるとき)
/// `ENOMOTO_T_MIP_GR_AUTO_MIN_FRAC` (既定 0.01): 一般整数の列が固定されていない列のこの割合に満たない問題では働かせない。
/// nursesched-sprint02 (一般整数 20 / 10250 列) では根の頂点が変わって根のヒューリスティクスが外れ (71、既定は 59)、再スタートで
/// 固まる列が 7000 本前後 -> 1382 本になり時間切れになる回がある。mzzv42z は 235 / 11717 列 (2.0%)
pub(crate) fn implbd_genint_for(is_int: &[bool], lo: &[f64], up: &[f64]) -> bool {
    if !gr_auto() {
        return false;
    }
    let ngen = (0..is_int.len()).filter(|&k| is_int[k] && lo[k] < up[k] && !(lo[k] >= 0.0 && up[k] <= 1.0)).count();
    let nfree = (0..is_int.len()).filter(|&k| lo[k] < up[k]).count();
    ngen > 0 && ngen as f64 >= tunable!("ENOMOTO_T_MIP_GR_AUTO_MIN_FRAC", 0.01, f64) * nfree as f64
}

/// `ENOMOTO_MIP_GR_RESTART` (試作、inv/mzzv42z-r4-20261021、既定では使わない): 一般整数のある問題 ([`implbd_genint_for`] が真、
/// 既定の P3 と同じ条件) だけで、再スタートと根の扱いを変える ((1) と (3)。(2) は `=readd` のときだけ)。
/// (1) `ENOMOTO_MIP_RESTART_REF` (再スタート前の暫定解を参照解として残す・打ち切り値に届かない解を暫定解にしない・下界を報告する)、
/// (2) `ENOMOTO_MIP_RESTART_CUTS_READD` (再スタート前に効いていたカットを問題の行にせず、根の LP の前にカットの行として足す)、
/// (3) `ENOMOTO_MIP_BG_CUTS_REHEUR=rins` (背景のカットで置き換えた根の LP の解で、丸めと RINS をもう一度回す)。
/// mzzv42z (4 スレッド・60 秒) では、根の RENS が固定率 0.35〜0.43 に崩れる回 (根のカットの 1 ラウンド目の LP が時間の上限に
/// 当たって根のカットが 1〜2 ラウンドで止まる回) でも背景のカットの後の RINS で -20400 台が出て、再スタートで暫定解を
/// 捨てて根の LP を解き直す (6〜19 秒) のをやめる (analysis/mzzv42z_r4_20261021.md)
/// 再スタートで持ち込むカットの扱いの修正 (既定で使う。`ENOMOTO_MIP_NO_RESTART_CUTS_FIX` で使わない)。(1) 前の求解が readd と
/// 決めたら新しい求解でも根の LP の前にカットの行として戻す (以前はカットプールに入るだけで、exp-1-500-5-5 では根の LP が毎周
/// 2.87e4 から作り直していた)、(2) プールの経路のカットは、大域的に固定された列を代入してから持ち込む (以前は前処理で固定された
/// 列を含むカットを捨てていて、p200x1188c では持ち込んだ 67 本が全部捨てられていた)。
/// 手元 16 スレッド・60 秒 ([`restart_cuts_subst`] と合わせて): exp-1-500-5-5 10.4 -> 4.3 秒、p200x1188c 3.0 -> 2.5 秒
/// (analysis/restart_cuts_fix_20261028.md)
pub(crate) fn restart_cuts_fix() -> bool {
    env_str!("ENOMOTO_MIP_NO_RESTART_CUTS_FIX").is_none()
}

/// `ENOMOTO_MIP_RESTART_CUTS_ROWS` (試作、既定では使わない。以前の `ENOMOTO_MIP_RESTART_CUTS_FIX` の修正 3): 再スタートで新たに
/// 固定される整数列が多い (`ENOMOTO_T_MIP_RESTART_CUTS_ROWS_FRAC`) ときは readd でも問題の行として持ち込む。前処理がカットの行の
/// 列を消せなくなり、p200x1188c では前処理後 266 行 -> 947 行になって遅くなりうる (手元 16 スレッドで 3.4 -> 4.8 秒の回)。
/// [`restart_cuts_subst`] で写せば要らない
pub(crate) fn restart_cuts_rows() -> bool {
    env_str!("ENOMOTO_MIP_RESTART_CUTS_ROWS").is_some()
}

/// `ENOMOTO_MIP_RESTART_HCM_KEEP` (試作): 再スタート前の求解が HiGHS 流のカット管理を自動で選んでいたら、再スタート後の求解でも
/// 最初から使う ([`RESTART_HCM`])。以前は新しい求解でその判定が立たず、exp-1-500-5-5 では 1 回目の再スタートだけが readd の
/// 経路で、2 回目からはカットを問題の行として持ち込み (行が 550 -> 893)、木のカットの扱いも既定に戻っていた
pub(crate) fn restart_hcm_keep() -> bool {
    env_str!("ENOMOTO_MIP_RESTART_HCM_KEEP").is_some()
}

/// 再スタートのカットを代入で写す (既定で使う。`ENOMOTO_MIP_NO_RESTART_CUTS_SUBST` で使わない): 再スタート前のカットを前処理後の
/// 空間に写すとき、前処理で消された列を含むカットを捨てずに、後処理の手順を代入して写す ([`map_restart_cut_subst`])。
/// p200x1188c は再スタートの前処理が列を別の列で置き換える (代入 47 手) ので、持ち込んだ 61〜67 本が全部捨てられていた (今は全部写す)
pub(crate) fn restart_cuts_subst() -> bool {
    env_str!("ENOMOTO_MIP_NO_RESTART_CUTS_SUBST").is_none()
}

/// 再スタート前のカット `c x <= r` (前処理前の空間) を、前処理後の問題 `prob` の列だけのカットに写す。尺度 (`x = d x'`) を
/// 掛けた後、後処理の手順をログの順に代入する (後処理は逆順に当てるので、前の手順の式に出てくる列は後の手順で決まる):
/// 代入 (`Sub`) は線形の式で置き換え、平行列の併合は `x_kept + s x_var` の形 (消した列の係数が残した列の係数の `s` 倍) の
/// ときだけ残した列に移す。強制列と、形の合わない平行列を含むカットは捨てる。最後に前処理後の問題で固定された列を右辺に移す
fn map_restart_cut_subst(c: &[(usize, f64)], r: f64, p: &MipProblem, prob: &MipProblem, post: &[crate::presolve::PostsolveStep], d: &[f64]) -> Option<(Vec<(usize, f64)>, f64)> {
    use crate::presolve::PostsolveStep;
    let mut coef: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
    for &(j, v) in c {
        if j >= p.n {
            return None;
        }
        *coef.entry(j).or_insert(0.0) += v * d.get(j).copied().unwrap_or(1.0);
    }
    let mut rhs = r;
    for step in post {
        match step {
            PostsolveStep::Sub(s) => {
                let Some(a) = coef.remove(&s.var).filter(|&a| a != 0.0) else { continue };
                let f = a / s.coeff;
                rhs -= f * s.rhs;
                for &(k, v) in &s.terms {
                    *coef.entry(k).or_insert(0.0) -= f * v;
                }
            }
            PostsolveStep::ParallelCol(s) => {
                let av = coef.get(&s.var).copied().unwrap_or(0.0);
                if av == 0.0 {
                    continue;
                }
                let ak = coef.get(&s.kept).copied().unwrap_or(0.0);
                if (av - s.s * ak).abs() > 1e-9 * (1.0 + av.abs()) {
                    return None;
                }
                coef.remove(&s.var);
            }
            PostsolveStep::Forcing(fc) => {
                if coef.get(&fc.var).is_some_and(|&a| a != 0.0) {
                    return None;
                }
            }
            PostsolveStep::Slack(sc) => {
                if coef.get(&sc.var).is_some_and(|&a| a != 0.0) {
                    return None;
                }
            }
        }
    }
    let mut out: Vec<(usize, f64)> = Vec::new();
    for (j, a) in hm_entries(&coef) {
        if a.abs() <= 1e-12 {
            continue;
        }
        if prob.col_lo[j] < prob.col_up[j] {
            out.push((j, a));
        } else {
            rhs -= a * prob.col_lo[j];
        }
    }
    if out.is_empty() || !rhs.is_finite() {
        return None;
    }
    out.sort_unstable_by_key(|&(j, _)| j);
    Some((out, rhs))
}

/// 根の被約費用の早めの記録 (既定で使う。`ENOMOTO_MIP_NO_EARLY_REDCOST` で使わない): 根のヒューリスティクスの前 (根のカットの後) と
/// 背景のカットで LP を置き換えた後にも根の被約費用を記録して固定する。以前は根のヒューリスティクスと背景のカットの後にしか
/// 記録しなかったので、根の途中で見つかった解 (FJ・Local-MIP・ヒューリスティクス) で被約費用による固定が進まなかった
pub(crate) fn early_redcost_on() -> bool {
    env_str!("ENOMOTO_MIP_NO_EARLY_REDCOST").is_none()
}

/// 早めの再スタート (既定で使う。`ENOMOTO_MIP_NO_EARLY_RESTART` で使わない): 根のヒューリスティクスで暫定解が良くなり、
/// 新たに固定された整数列が根の始めに自由だった整数列の `ENOMOTO_T_MIP_EARLY_RESTART_FRAC` (0.025) 以上なら、残りの根の
/// ヒューリスティクスを飛ばして再スタートする。[`early_redcost_on`] と組んで効く (固定が根の途中で進むため)。
/// 負けている 21 問 (16 スレッド・60 秒・2 回): 解けた回数 35 -> 37、SGM 21.99 -> 20.89 秒
/// (p200x1188c 6.5 -> 3.4 秒、h80x6320d 52 -> 43 秒、exp-1-500-5-5 15.4 -> 12.6 秒、swath1 23.5 -> 31.1 秒)
pub(crate) fn early_restart_on() -> bool {
    env_str!("ENOMOTO_MIP_NO_EARLY_RESTART").is_none()
}

pub(crate) fn gr_restart_for(p: &MipProblem) -> bool {
    env_def!("ENOMOTO_MIP_GR_RESTART", "1").is_some() && implbd_genint_for(&p.is_int, &p.col_lo, &p.col_up)
}

/// [`gr_restart_for`] で (2) `ENOMOTO_MIP_RESTART_CUTS_READD` も使うか (`ENOMOTO_MIP_GR_RESTART=readd` のときだけ。
/// カットを問題の行にしないと再スタート後の根のカットが弱くなる問題がある: mik-250-20-75-4 は -53829 -> -53948、7.4〜7.9 秒 -> 9.5〜10.5 秒。
/// mzzv42z では (2) がなくても再スタート後 12〜13 秒で木に入る (あれば 8〜12 秒))
pub(crate) fn gr_restart_readd_for(p: &MipProblem) -> bool {
    env_def!("ENOMOTO_MIP_GR_RESTART", "1") == Some("readd") && gr_restart_for(p)
}

/// [`gr_restart_for`] で、2 回目以降の再スタートの割合の分母を根の始めに自由だった整数列の数にするか
/// (`ENOMOTO_MIP_RESTART_FRAC_FREE` と同じ。`ENOMOTO_MIP_GR_RESTART=ff` のときだけ)。HiGHS の `percentageInactiveIntegers` は
/// 再スタート後の (前処理で縮んだ) 問題の整数列に対する割合なので、最適解を得た後に 2〜4 回再スタートし、そのたびに根のカットで
/// 下界を上げる (mzzv42z: -20811 -> -20683 -> -20640 -> 1 ノードで証明)。分母が全部の整数列 (11717 本) だと、こちらは再スタート後に
/// 新たに固定した 287〜304 列が閾値 (293) に届かず 2 回目の再スタートが起きない (analysis/mzzcmp_20261023.md)
pub(crate) fn gr_restart_ff_for(p: &MipProblem) -> bool {
    env_def!("ENOMOTO_MIP_GR_RESTART", "1") == Some("ff") && gr_restart_for(p)
}

/// 再スタート前の暫定解を参照解として残すか ([`gr_restart_for`] の (1)。全体で使う試作 `ENOMOTO_MIP_RESTART_REF` は 2026-11-18 に削除)
pub(crate) fn restart_ref_for(p: &MipProblem) -> bool {
    gr_restart_for(p)
}

/// 試作 `ENOMOTO_MIP_TIMEDEP` = 文字の並び (既定では使わない、analysis/timedep_20261114.md): 経過時間で決めていた分かれ道・量を
/// 手間 (LP の反復・入れ替えの数・ラウンド数・触った非零の数) で決める。時間の上限は安全網として大きくして残す。
/// - `m`: 多点分離 (MPSEP `o`) の最適面の上の移動を、入れ替えの数の上限 max(`ENOMOTO_T_MIP_TIMEDEP_MPSEP_P0` (既定 16、初版は 125),
///   `ENOMOTO_T_MIP_TIMEDEP_MPSEP_C` (既定 0、初版は 0.03) x 根の LP の反復) で止める (時間は `ENOMOTO_T_MIP_TIMEDEP_MPSEP_SAFE` 秒 (既定 1) の安全網)
/// - `p`: 根のカットのループの第 2 段階の上限を、手間 (カットの LP の反復 / 根の LP の反復 + 0.05 x ラウンド数) で第 1 段階の
///   `ENOMOTO_T_MIP_CUT_PHASE2_TIME_MULT` 倍 (最低 `ENOMOTO_T_MIP_TIMEDEP_PHASE2_MIN` (既定 1)) にする (時間は 10 倍の安全網)
/// - `l`: 根の局所探索を、触った非零の数 `ENOMOTO_T_MIP_TIMEDEP_LS_RATE` (既定 1.5e8) x 今の時間の枠の秒 で止める (時間は 10 倍の安全網)
/// - `r`: 根の LP (基底のない求解) の同時実行の勝ち手を、二段解法の反復の上限 (`ENOMOTO_T_MIP_TIMEDEP_RACE_C` (既定 1) x (行 + 列)) 以内に
///   結論が出たかで決める (出なければ内点法 + クロスオーバーの頂点。simplex/mip_lp.rs の `solve_race_ext`)
/// - `c`: カット選びの比べ (CUTSEL_PROBE) の伸びの速さ・合計の予算を手間で数える (予算 `ENOMOTO_T_MIP_TIMEDEP_PROBE_WORK` (既定 5) 回分の根の LP)
/// - `w`: 根のカットのループの上限 (CPU 時間で全体の 0.1 倍) を手間 (`ENOMOTO_T_MIP_CUT_WORK_CAP` の形、既定 12。時間は 2 倍の安全網) にする
/// - `n`: 木の大近傍探索の予算を木の LP の反復の割合で数え、1 回のサブ MIP を LP の反復の上限で止める
/// - `N` (`n` と一緒に): 木の大近傍探索のサブ MIP の時間の上限を残り時間によらず `submip_time_max` (既定 6 秒) にする
/// - `e`: 木の中の再スタートのガード (`TREEEFF` の `r`) の木の秒・根の秒を LP の反復で数える
/// - `t`: 木の中の分離をするか・続けるか (`tree_sep_budget_ok`) を秒でなく LP の反復で比べる
/// - `y`: 前処理の対称性の探り (`ENOMOTO_MIP_SYMPRE=probe`) を手間の上限だけで止める (時間は 10 倍の安全網)
/// - `b`: 割合で決める量 (時間の上限 x 割合) の基準を、前処理・再スタートの後の残り時間でなく元の時間の上限にする ([`solver::MipParams::frac_tl`])
/// - `u`: 外側の求解のサブ MIP (根の RENS・RINS など。木の大近傍探索は `n`) を時間でなく LP の反復の上限で止める (時間は 10 倍の安全網。
///   `heuristics::Solver::submip_iter_cap_u`、係数 `ENOMOTO_T_MIP_TIMEDEP_SUBMIP_C` 既定 5)
/// 試作 `ENOMOTO_MIP_TICKS` (既定では使わない。analysis/timedep_20261114.md の 11 節): 秒の代わりに手間の単位 ([`crate::ticks`]、
/// 当てはめた重みで 1 手間 ≒ 1 ns) で、時間の規則と同じ式で決める。文字:
/// - `t`: 木の分離の予算 (`tree_sep_budget_ok`・`node_cut_adapt_due`・`nca_lp_budget_ok`)
/// - `n`: 木の大近傍探索の予算と 1 回の上限 (サブ MIP は手間の期限で止める)
/// - `w`: [`cap_clock`] (スレッドの CPU 時間) を手間の秒にする (根のカットのループ・CUTSEL_PROBE・局所探索など)
/// - `u`: 外側の求解のサブ MIP (根の RENS など) を時間の規則と同じ量の手間の期限で止める
/// - `m`: MPSEP の最適面の移動を入れ替えの数 (`ENOMOTO_T_MIP_TIMEDEP_MPSEP_P0`、既定 16) で止める (時間は 1 秒の安全網。TIMEDEP の `m` と同じ)
/// - `c`: CUTSEL_PROBE の比べの経過・打ち切り・LP の上限を手間の秒にする
/// - `f`: 前処理の早い FJ・Local-MIP (1 本のとき) の全体の上限と Local-MIP の 1 切れを手間で数える
/// - `l`: 局所探索の時間の上限を同じ量の手間の上限にする
/// - `s`: 手間の期限の中のサブ MIP の中で作るサブ MIP (入れ子) の時間の上限を、壁時計の残りでなく手間の残りから同じ式で決め、手間の期限で止める
///   (時間の上限は 10 倍の安全網)
/// - `h`: LP の重い求解の判定 (`ENOMOTO_MIP_LPSCALE` の `h`: 尺度なしの求解が `ENOMOTO_T_MIP_LPSCALE_HEAVY_SECS` (1) 秒以上なら尺度をかけた双子に
///   切り替え、以後に作る LP も双子を持つ) を、その求解の手間で決める
/// - `k`: ノードの分離の時間の割合の判定のうち `t` の外のもの (HiGHS 流のカット管理のノードの分離・TREESEP2・FC の分離) を手間で比べる
/// - `r`: 根の LP の同時解き (`solve_race_ext`) の勝ち手を、先に終わった方でなく二段解法の反復の上限で決める (TIMEDEP の `r` と同じ)
/// - `j`: FJ・Local-MIP のスレッド (4 スレッドの根) の時計を手間にし (FJ と Local-MIP の取り分・切れ・上限。上限は元の時間の上限から)、
///   主とは手間の門 ([`crate::ticks::Gate`]) で受け渡す: 主は受け渡し箱を見る所でスレッドが許された手間に着くまで待って読み、
///   前に許してからの主のスレッドの手間を次に許す。主の暫定解の有無はスレッドに伝えない。LMBOOST は使わない
/// 試作 `ENOMOTO_MIP_DETHASH` (既定では使わない。analysis/timedep_20261114.md の 12 節): 結果に効く `HashMap` の走査を鍵の順にする
/// (既定の `RandomState` はプロセスごとに種が変わり、走査順で implied bound カットの順・同値の組の順・足し合わせの順が変わる)。
///
/// **cand1118 (2026-11-16、analysis/treeclose_20261114.md 9.51 節) で既定にした: 値 `1`。`0`・`off` で 0.1.7 までの動き (使わない)。**
pub(crate) fn dethash() -> bool {
    static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V.get_or_init(|| std::env::var("ENOMOTO_MIP_DETHASH").map_or(true, |v| !v.is_empty() && v != "0" && v != "off"))
}

/// `m` の組を取り出す ([`dethash`] なら鍵の順に並べる。そうでなければ今までと同じ走査順)。
pub(crate) fn hm_entries<K: Ord + Copy + std::hash::Hash + Eq, V: Copy, S: std::hash::BuildHasher>(m: &std::collections::HashMap<K, V, S>) -> Vec<(K, V)> {
    let mut v: Vec<(K, V)> = m.iter().map(|(k, x)| (*k, *x)).collect();
    if dethash() {
        v.sort_by(|a, b| a.0.cmp(&b.0));
    }
    v
}

///
/// **cand1118 (2026-11-16、analysis/treeclose_20261114.md 9.51 節) で既定にした: 値 `ntwumcflshk`。`0`・`off` で 0.1.7 までの動き (使わない)。**
pub(crate) fn ticks_on(c: char) -> bool {
    env_def!("ENOMOTO_MIP_TICKS", "ntwumcflshk").is_some_and(|v| v.contains(c))
}

/// 試作 `ENOMOTO_MIP_PCL` の文字 (既定では使わない、analysis/disj_20261119.md の 11 節): `a` 打ち切った強分岐の子の値を下限の観測に・
/// lookahead に数えない、`b` 観測のない列の費用を似た列で埋める、`c` 信頼性をばらつきで決める、`d` 解き直した後の上がり幅を観測にする
pub(crate) fn pcl(c: char) -> bool {
    env_str!("ENOMOTO_MIP_PCL").is_some_and(|v| v.contains(c))
}

/// 試作 `ENOMOTO_MIP_ROOTSB` の文字の並び (既定では使わない、`Solver::select_branch_inner`、analysis/disj_20261119.md の 9 節)
pub(crate) fn rootsb_flags() -> Option<&'static str> {
    env_str!("ENOMOTO_MIP_ROOTSB").filter(|v| !v.is_empty() && *v != "0")
}

/// `ENOMOTO_MIP_TIMEDEP` の文字 (analysis/timedep_20261114.md)。既定 `b` (割合で決める量の基準を元の時間上限にする。2026-11-21 に
/// `ENOMOTO_MIP_DET1T=onil` と組で既定化、analysis/det1t_20261120。`=0` で以前の形。ほかの文字を指定すると `b` は外れるので、続けて使うなら `bw` のように足す)
pub(crate) fn timedep(c: char) -> bool {
    env_def!("ENOMOTO_MIP_TIMEDEP", "b").is_some_and(|v| v.contains(c))
}

/// このスレッドの本当の CPU 時間 (秒、Linux 以外は経過時間)。[`cap_clock`] と違い、`ENOMOTO_MIP_TICKS` の `w` でも手間にしない (診断用)
pub(crate) fn real_thread_cpu() -> f64 {
    #[cfg(target_os = "linux")]
    {
        #[repr(C)]
        struct Timespec {
            tv_sec: i64,
            tv_nsec: i64,
        }
        extern "C" {
            fn clock_gettime(clk: i32, tp: *mut Timespec) -> i32;
        }
        let mut ts = Timespec { tv_sec: 0, tv_nsec: 0 };
        // SAFETY: `ts` は有効な書き込み先で、clock_gettime はそこに書くだけ
        if unsafe { clock_gettime(3, &mut ts) } == 0 {
            return ts.tv_sec as f64 + ts.tv_nsec as f64 * 1e-9;
        }
    }
    static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    EPOCH.get_or_init(std::time::Instant::now).elapsed().as_secs_f64()
}

thread_local! {
    /// 診断 [`wc_mark`]: 前の印の (本当の CPU 秒, 手間の合計, 部分ごとの手間)
    static WC_LAST: std::cell::Cell<Option<(f64, f64, [u64; crate::ticks::NK])>> = const { std::cell::Cell::new(None) };
}

/// 診断 `ENOMOTO_MIP_WORKCOUNT_DIAG` (analysis/workcount_20261125、解き方は変えない): 前の印からの本当の CPU 時間の増えと、手間 (重み付き、秒) の増えを
/// `WCDIAG` の行に出す。手間の数えていない処理を、印の間の比 (CPU / 手間) で探す
pub(crate) fn wc_mark(label: &str, submip: bool) {
    if env_str!("ENOMOTO_MIP_WORKCOUNT_DIAG").is_none() {
        return;
    }
    let (c, t, r) = (real_thread_cpu(), crate::ticks::total() * 1e-9, crate::ticks::raw());
    if let Some((c0, t0, r0)) = WC_LAST.with(|w| w.get()) {
        let parts: Vec<String> = (0..crate::ticks::NK).filter(|&k| r[k] != r0[k]).map(|k| format!("{} {}", crate::ticks::NAMES[k], r[k].wrapping_sub(r0[k]))).collect();
        eprintln!("WCDIAG {}{label}: cpu {:.3} ticks {:.3} ratio {:.2} | {}", if submip { "sub " } else { "" }, c - c0, t - t0, (c - c0) / (t - t0).max(1e-9), parts.join(" "));
    }
    WC_LAST.with(|w| w.set(Some((c, t, r))));
}

pub(crate) fn cap_clock() -> f64 {
    // 試作 `ENOMOTO_MIP_TICKS` の `w`: このスレッドの CPU 時間の代わりに手間の単位 ([`crate::ticks`]、1 手間 ≒ 1 ns) の秒を返す
    // (根のカットのループ・CUTSEL_PROBE・局所探索などの CPU 時間の上限がまとめて手間になる)
    if ticks_on('w') {
        return crate::ticks::total() * 1e-9;
    }
    #[cfg(target_os = "linux")]
    {
        if env_str!("ENOMOTO_MIP_WALL_CAPS").is_none() {
            #[repr(C)]
            struct Timespec {
                tv_sec: i64,
                tv_nsec: i64,
            }
            extern "C" {
                fn clock_gettime(clk: i32, tp: *mut Timespec) -> i32;
            }
            const CLOCK_THREAD_CPUTIME_ID: i32 = 3;
            let mut ts = Timespec { tv_sec: 0, tv_nsec: 0 };
            // SAFETY: `ts` は有効な書き込み先で、clock_gettime はそこに書くだけ
            if unsafe { clock_gettime(CLOCK_THREAD_CPUTIME_ID, &mut ts) } == 0 {
                return ts.tv_sec as f64 + ts.tv_nsec as f64 * 1e-9;
            }
        }
    }
    static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    EPOCH.get_or_init(std::time::Instant::now).elapsed().as_secs_f64()
}

/// 求解の経過の記録 (ベンチマークの指標用: 最初の暫定解の時刻と目的値、根の下界)。最小化形・定数項込みの値で、
/// サブ MIP の中のものは記録しない。プロセスで 1 つなので、同じプロセスで同時に複数の求解をすると混ざる (診断用)。
pub(crate) struct RunStats {
    pub(crate) t0: Option<std::time::Instant>,
    /// 最初の暫定解 (求解の開始からの秒, 目的値)。
    pub(crate) first_sol: Option<(f64, f64)>,
    /// 根の切除平面ループを終えた時点の LP の目的値 (再スタート後の根があれば最後のもの)。暫定解の目的値で上から切り詰める ([`record_root_bound`])。
    pub(crate) root_bound: Option<f64>,
    /// 最初の根のヒューリスティクスを始めた時刻 (求解の開始からの秒。[`root_submip_ref_secs`] の基準)。
    pub(crate) root_heur_t: Option<f64>,
    /// 最後に暫定解が良くなった時刻 (求解の開始からの秒。`ENOMOTO_MIP_ROBUST_LNSFREQ` の停滞の判定に使う)。
    pub(crate) last_sol: Option<f64>,
    /// [`record_incumbent`] で受け取った最良の目的値 (スレッドごとの暫定解は全体の最良より悪いことがあるので、全体で良くなったときだけ `last_sol` を進める)。
    pub(crate) best_z: f64,
}

pub(crate) static RUN_STATS: std::sync::Mutex<RunStats> = std::sync::Mutex::new(RunStats { t0: None, first_sol: None, root_bound: None, root_heur_t: None, last_sol: None, best_z: f64::INFINITY });

/// 試作 `ENOMOTO_MIP_OBJTARGET=文字` (analysis/objtarget_20261119/、既定では使わない): FJ (`f`)・Local-MIP (`l`) に「目的値 ≤ T」の目標を置く
pub(crate) fn objtarget(c: char) -> bool {
    env_str!("ENOMOTO_MIP_OBJTARGET").is_some_and(|v| v.contains(c))
}

/// [`objtarget`] の目標の材料: (下界 L = 根の切除平面ループの後の LP の値, 上界 U = 全体の最良の暫定解 (なければ +inf))。
/// どちらも外側の求解の目的値 (定数項を含む)。根がまだ終わっていなければ `None`
pub(crate) fn objtarget_bounds() -> Option<(f64, f64)> {
    let s = RUN_STATS.lock().ok()?;
    let l = s.root_bound?;
    l.is_finite().then_some((l, s.best_z))
}

/// 暫定解の記録 ([`RUN_STATS`])。最初のものだけを残す。
pub(crate) fn record_incumbent(z: f64) {
    if let Ok(mut s) = RUN_STATS.lock() {
        let t = s.t0.map_or(0.0, |t0| t0.elapsed().as_secs_f64());
        if s.first_sol.is_none() {
            s.first_sol = Some((t, z));
        }
        if solver::better_obj(z, s.best_z, z) {
            s.best_z = z;
            s.last_sol = Some(t);
        }
    }
}

/// 最後に暫定解が良くなってからの秒 (外側の求解の時計。暫定解がまだなければ `None`)。
pub(crate) fn secs_since_last_incumbent() -> Option<f64> {
    let s = RUN_STATS.lock().ok()?;
    let t0 = s.t0?;
    s.last_sol.map(|t| t0.elapsed().as_secs_f64() - t)
}

/// 外側の求解 ([`solve_mip`]) の始まりからの経過秒 ([`RUN_STATS`] の `t0`。再スタートを通して同じ時計)。
pub(crate) fn outer_elapsed() -> Option<f64> {
    RUN_STATS.lock().ok().and_then(|s| s.t0).map(|t0| t0.elapsed().as_secs_f64())
}

/// 最初の根のヒューリスティクスを始めた時刻を記録する (2 回目以降は何もしない)。
pub(crate) fn record_root_heur_start() {
    if let Ok(mut s) = RUN_STATS.lock() {
        if s.root_heur_t.is_none() {
            s.root_heur_t = s.t0.map(|t0| t0.elapsed().as_secs_f64());
        }
    }
}

/// 根の時間の基準の秒 (`ENOMOTO_MIP_SUBCAP` の [`super::solver::subcap_root_secs`]): 最初の根のヒューリスティクスを始めた時刻 (記録がなければ今の経過秒)。
pub(crate) fn root_submip_ref_secs() -> Option<f64> {
    RUN_STATS.lock().ok().and_then(|s| s.root_heur_t).or_else(outer_elapsed)
}

/// 根の下界の記録 ([`RUN_STATS`])。暫定解があれば、その目的値 (`best_z`) より大きくは記録しない (`ENOMOTO_MIP_ROOT_BOUND_CLAMP=0` で
/// 切り詰めない、元の記録)。暫定解を得た後の根 (再スタートの後の根や、被約費用の固定の後に解き直した根の LP) は
/// 「暫定解より良い解」だけを残した問題 (打ち切り値による被約費用の固定、刈り込みの上限) の下界で、暫定解が最適なら
/// その LP の値は最適値を越えうる (nw04 で 16866.37 > 16862、analysis/nw04_bound_20261106.md)。元の問題の下界は
/// min(暫定解の目的値, その LP の値) なので、ここで切り詰める
pub(crate) fn record_root_bound(z: f64) {
    if let Ok(mut s) = RUN_STATS.lock() {
        let clamp = env_str!("ENOMOTO_MIP_ROOT_BOUND_CLAMP").is_none_or(|v| v != "0");
        s.root_bound = Some(if clamp && s.best_z.is_finite() { z.min(s.best_z) } else { z });
    }
}

/// 木の探索に使えるスレッドの数の上限: このプロセスが使える CPU の数 (Linux では CPU affinity と cgroup の割り当てを
/// 反映する `std::thread::available_parallelism`)。環境変数 `ENOMOTO_MIP_MAX_THREADS` でさらに小さくできる。
pub fn max_threads() -> usize {
    // 診断用: `ENOMOTO_MIP_DIAG_HW_THREADS` で使える CPU の数を上書きする (CPU の少ない計算機で多スレッドの振る舞いを再現する)
    let hw = env_str!("ENOMOTO_MIP_DIAG_HW_THREADS")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1));
    let cap = env_str!("ENOMOTO_MIP_MAX_THREADS").and_then(|v| v.parse::<usize>().ok()).filter(|&v| v >= 1).unwrap_or(usize::MAX);
    hw.min(cap).max(1)
}

/// スレッドの数を指定しないときの既定値: このプロセスが使える CPU ([`max_threads`]) の数 + 1 の半分 (HiGHS の
/// `threads = 0` と同じ。論理コアが 4 なら 2、8 なら 4)。
pub fn default_threads() -> usize {
    max_threads().div_ceil(2).max(1)
}

/// (混合) 整数最適化問題を解く。整数変数が 1 つもなければ `solve_lp` を 1 回呼ぶだけ。
pub fn solve_mip(
    variables: &[VariableData],
    objective: &Objective,
    constraints: &[ConstraintRow],
    root_solver: RootSolver,
    opts: LpOptions,
    settings: MipSettings,
) -> SolveResult {
    // 時間の上限はここから数える (問題の組み立て・畳み込みの判定の時間も含める。[`fold::solve_folded_any`] の後の `dispatch` を参照)
    let t_entry = std::time::Instant::now();
    let has_discrete = variables.iter().any(|v| v.vtype != VarType::Continuous);
    if !has_discrete {
        // 制限時間: 期限を過ぎると各解法は結論なしで戻る ([`crate::cancel::with_deadline`])。そのときは `TimeLimit`
        let deadline = settings.time_limit.filter(|t| t.is_finite()).map(|t| std::time::Instant::now() + std::time::Duration::from_secs_f64(t.max(0.0)));
        let mut r = crate::cancel::with_deadline(deadline, || solve_lp(variables, objective, constraints, root_solver, opts));
        if r.status == Status::NotSolved && deadline.is_some_and(|d| std::time::Instant::now() >= d) {
            r.status = Status::TimeLimit;
        }
        return r;
    }
    let p = MipProblem::from_model(variables, objective, constraints);
    if let Ok(mut st) = RUN_STATS.lock() {
        *st = RunStats { t0: Some(std::time::Instant::now()), first_sol: None, root_bound: None, root_heur_t: None, last_sol: None, best_z: f64::INFINITY };
    }
    // 試作 `ENOMOTO_MIP_LPSCALE=h`: 重い LP を見て立てた印は外側の求解ごとに下ろす
    crate::simplex::mip_lp::lpscale_disarm();
    // `ENOMOTO_MIP_WORK_LIMIT` の手間の数えは外側の求解ごとに 0 から
    solver::WORK_BASE.store(0, std::sync::atomic::Ordering::Relaxed);
    // 既定値 < 環境変数 < 引数の順に優先する。
    let env_f64 = |v: Option<&str>| v.and_then(|s| s.parse::<f64>().ok());
    let d = MipParams::default();
    let params = MipParams {
        verbose: env_str!("ENOMOTO_MIP_LOG").is_some(),
        rel_gap: settings.rel_gap.or(env_f64(env_str!("ENOMOTO_MIP_REL_GAP"))).unwrap_or(d.rel_gap),
        time_limit: settings.time_limit.or(env_f64(env_str!("ENOMOTO_MIP_TIME_LIMIT"))).unwrap_or(d.time_limit),
        node_limit: settings.node_limit.or(env_f64(env_str!("ENOMOTO_MIP_NODE_LIMIT")).map(|v| v as u64)).unwrap_or(d.node_limit),
        // 未指定なら `opts.threads` (Python の `Model.solve` は未指定のとき [`default_threads`] を渡す)
        threads: settings.threads.or(env_f64(env_str!("ENOMOTO_MIP_TREE_THREADS")).map(|v| v as usize)).unwrap_or(opts.threads.max(d.threads)).clamp(1, max_threads()),
        distinguish: opts.distinguish_infeasible_unbounded,
        // 診断用 (`ENOMOTO_MIP_CUTOFF`): 打ち切り値を元の目的の向きで与える (この値以上の下界のノードを捨てる)。
        // 最適値が分かっている問題で、暫定解を見つける運の良し悪しと証明の時間を切り分けるために使う
        // (pk1 の揺れの調査。最適値 11 の整数目的なら 10.5 を与えると、暫定解 11 を持って始めたのと同じ木になる)
        cutoff: env_f64(env_str!("ENOMOTO_MIP_CUTOFF")).map_or(d.cutoff, |v| p.sense_sign * v),
        // 乱数の種: 引数 (`Model.solve(seed=...)`) があればそれ、なければ環境変数 `ENOMOTO_T_MIP_SEED`、なければ 0 ([`solver::default_seed`])
        seed: settings.seed.unwrap_or(d.seed),
        ..d
    };
    let params = MipParams { tl_base: params.time_limit, ..params };
    // 診断用 (`ENOMOTO_MIP_DEBUG_AS_SUBMIP`): 書き出したサブ MIP (`ENOMOTO_MIP_SUBMIP_DUMP`) を、根のサブ MIP と同じ形
    // (入れ子の深さ 1・1 スレッド・ギャップ 1e-4) で解く。打ち切り値は `ENOMOTO_MIP_CUTOFF` で与える
    let params = if env_str!("ENOMOTO_MIP_DEBUG_AS_SUBMIP").is_some() {
        heuristics::SUBMIP_DEPTH.with(|d| d.set(1));
        MipParams { submip: true, threads: 1, rel_gap: params.rel_gap.max(1e-4), abs_gap: params.abs_gap.max(1e-6), ..params }
    } else {
        params
    };
    // 基底のない LP の求解 (根・再スタート後の根) を `threads` 本で同時実行にする (木の探索の従のスレッドを始めたら、
    // またヒューリスティクスのサブ MIP の中では 1)
    // 診断用 (`ENOMOTO_MIP_START_FILE`): 最初の暫定解を与える (元の問題の列の順に 1 行 1 値)。再スタートで暫定解を
    // 引き継ぐ経路 (`RESTART_SOL`) に置くので、前処理後の問題で実行可能なときだけ使われる。主側 (解を見つける) と
    // 双対側 (証明) の切り分けに使う (mzzv42z・neos-960392 の調査)。
    if let Some(f) = env_str!("ENOMOTO_MIP_START_FILE") {
        match std::fs::read_to_string(f) {
            Ok(s) => {
                let x: Vec<f64> = s.lines().filter_map(|l| l.trim().parse().ok()).collect();
                let ok = x.len() == p.n;
                eprintln!(
                    "MIP_START_FILE: {} values (n = {}), objective {} feasible {}",
                    x.len(),
                    p.n,
                    if ok { p.sense_sign * p.objective(&x) } else { f64::NAN },
                    ok && p.is_feasible(&x, 1e-6)
                );
                if ok {
                    RESTART_SOL.with(|s| *s.borrow_mut() = Some(x));
                }
            }
            Err(e) => eprintln!("MIP_START_FILE: cannot read {f}: {e}"),
        }
    }
    // `ENOMOTO_MIP_THREAD_BUDGET`: 根の LP の同時実行は FJ のスレッドの分を 1 本引く
    let race_threads = if solver::thread_budget_on() && params.threads > 1 { params.threads - 1 } else { params.threads };
    let prev_race = crate::simplex::mip_lp::set_cold_race_threads(race_threads);
    // `ENOMOTO_MIP_G5SEQ` (既定): 1 スレッドでは基底のない大きい求解を二段解法 → 内点法 + クロスオーバーの順に回す
    let prev_seq = crate::simplex::mip_lp::set_seq_ipm(race_threads < 2 && !params.submip && crate::simplex::mip_lp::g5seq_on());
    // 根のサブ MIP・Feasibility Jump を別スレッドで回すかなどの判定 ([`solver::mip_threads`]) も、`ENOMOTO_MIP_THREADS` が
    // なければこのスレッド数にする
    let prev_mt = solver::MIP_THREADS_DEFAULT.swap(params.threads, std::sync::atomic::Ordering::Relaxed);
    let t_solve = std::time::Instant::now();
    // 診断用 (`ENOMOTO_MIP_NB_DIAG`): `set_basis` の置き場所と `nb_hint` の食い違いの集計 (analysis/basis_nb_diag_20261102.md)
    if crate::simplex::nb_diag::on() {
        crate::simplex::nb_diag::reset();
    }
    let dispatch = |q: &MipProblem, tl: f64| {
        let params = MipParams { time_limit: tl, ..params };
        if env_str!("ENOMOTO_MIP_PORTFOLIO").is_some() {
            solve_portfolio(q, params)
        } else if env_str!("ENOMOTO_MIP_PERMRACE").is_some() && params.threads >= 2 && !params.submip {
            solve_permrace(q, params)
        } else {
            solve_problem(q, params, env_str!("ENOMOTO_MIP_NO_PRESOLVE").is_none())
        }
    };
    // `ENOMOTO_MIP_FOLD` (2026-11-10 から既定、`0` で止める。analysis/drcr_20261110.md): 色の細分化で畳んで解き、元に戻す ([`fold`])
    let dispatch_np = |q: &MipProblem, tl: f64| solve_problem(q, MipParams { time_limit: tl, ..params }, false);
    // 試作 `ENOMOTO_MIP_ORDINV` の `c` ([`canonical_problem`]): 行・列を鍵の順に並べ直して解き、解を元の並びに戻す
    let canon = crate::presolve::ordkey::ordinv('c').then(|| canonical_problem(&p));
    let pr = canon.as_ref().map_or(&p, |(q, _)| q);
    // 診断用の開始解 (`ENOMOTO_MIP_START_FILE`) も並べ直す
    if let Some((_, cp)) = &canon {
        RESTART_SOL.with(|s| {
            if let Some(x) = s.borrow_mut().as_mut().filter(|x| x.len() == cp.len()) {
                *x = cp.iter().map(|&j| x[j]).collect();
            }
        });
    }
    // `ENOMOTO_MIP_VUBAGG` (既定で使う。`=0` で切る。analysis/vsscip_slow_20261118/g1): 双対の集約 ([`vub_dual_agg_pairs`])。畳み込み・前処理の前に
    // x_j を y に置き換えた問題を解き、解の x_j を y の値で埋める。畳み込み (既定) は平行な x を先に束ねて形を崩すので、その前に見る
    let vub = if env_def!("ENOMOTO_MIP_VUBAGG", "1").is_some() && !params.submip {
        let t0 = std::time::Instant::now();
        let pairs = vub_dual_agg_pairs(pr);
        if params.verbose {
            eprintln!("MIP: VUBAGG {} columns x = y from variable upper bound rows ({:.2}s)", pairs.len(), t0.elapsed().as_secs_f64());
        }
        // 試作 `ENOMOTO_T_MIP_VUBAGG_MIN_FRAC` = f (既定 0 = 従来どおり。analysis/regv019_20261119): まとめる列が全部の列の f に満たなければ
        // まとめない。働く問題でまとめる割合は neos-787933 99% (200〜470 秒 → 0.4 秒)・neos-4722843-widden 6.9% (中立) と、
        // physiciansched6-2 2.0% (前処理の後は同じ大きさになり、根のカットの道が変わって 4 スレッドで 3 種とも 0.61〜0.90 倍遅い)・
        // piperout-27 0.5%・neos8 0.07% (中立) に分かれる
        let min_frac = tunable!("ENOMOTO_T_MIP_VUBAGG_MIN_FRAC", 0.0, f64);
        let small = (pairs.len() as f64) < min_frac * pr.n as f64;
        if params.verbose && small && !pairs.is_empty() {
            eprintln!("MIP: VUBAGG not used ({} of {} columns < {min_frac})", pairs.len(), pr.n);
        }
        (!pairs.is_empty() && !small).then(|| (vub_dual_agg_problem(pr, &pairs), pairs))
    } else {
        None
    };
    let pr = vub.as_ref().map_or(pr, |(q, _)| q);
    // 試作 `ENOMOTO_MIP_BIGM_SPLIT` (既定では使わない。analysis/scipslow_20261119): 大 M の行の分解 ([`bigm_split_problem`])。列は変えないので解はそのまま
    let bigm = if env_str!("ENOMOTO_MIP_BIGM_SPLIT").is_some() && !params.submip {
        let t0 = std::time::Instant::now();
        let q = bigm_split_problem(pr);
        if params.verbose {
            eprintln!(
                "MIP: BIGM_SPLIT {} ({:.2}s)",
                q.as_ref().map_or("none".to_string(), |q| format!("rows {} -> {}, nonzeros {} -> {}", pr.m, q.m, pr.nnz, q.nnz)),
                t0.elapsed().as_secs_f64()
            );
        }
        q
    } else {
        None
    };
    let pr = bigm.as_ref().unwrap_or(pr);
    // 時間の上限から、入口からここまで (問題の組み立てなど) と畳み込みの判定 (畳まないと決めるまで) の時間を引いて渡す。引かなかった版では
    // highschool1-aigio (列 32 万) で畳み込みの判定に 2.7 秒かかり、その分だけ時間の上限を越えていた (4 スレッド 600 秒で 603〜606 秒。
    // analysis/hang_20261117/README.md)。`ENOMOTO_MIP_NO_ENTRY_DEADLINE` で引かない (直す前と同じ、切り分け用)
    let entry_off = env_str!("ENOMOTO_MIP_NO_ENTRY_DEADLINE").is_none();
    let left = |tl: f64| if entry_off && tl.is_finite() { (tl - t_entry.elapsed().as_secs_f64()).max(0.0) } else { tl };
    let mut r = match fold::solve_folded_any(pr, left(params.time_limit), params.verbose, &dispatch, &dispatch_np) {
        Some(r) => r,
        None => dispatch(pr, left(params.time_limit)),
    };
    if let (Some((_, pairs)), Some(x)) = (&vub, r.x.as_mut()) {
        for &(j, y, _) in pairs {
            x[j] = x[y];
        }
    }
    // 注意: `if let (Some(..), Some(x)) = (&canon, r.x.take())` と書くと、canon が None (既定) でも `r.x.take()` が評価されて
    // 解が捨てられる (af29a20a の形。既定ですべての問題が「最適・解なし」になっていた。analysis/wrong3470_20261114.md)
    if let Some((_, cp)) = &canon {
        if let Some(x) = r.x.take() {
            let mut y = vec![0.0; p.n];
            for (q, &j) in cp.iter().enumerate() {
                y[j] = x[q];
            }
            r.x = Some(y);
        }
    }
    // 利用者の打ち切り値より良くない解は返さない ([`patch_user_cutoff`])
    let r = if params.submip { r } else { patch_user_cutoff(r, params.cutoff, params.verbose) };
    if solver::end_diag() || params.verbose {
        eprintln!("MIP: solve returned {:?} after {:.2}s (time limit {:.2}s)", r.status, t_solve.elapsed().as_secs_f64(), params.time_limit);
    }
    // `ENOMOTO_MIP_SUBA_CUT` (試作): サブ MIP が打ち切り値より悪いとして捨てた解の数
    if params.verbose && solver::suba_cut() {
        eprintln!("MIP: SUBA_CUT rejected {} sub-MIP solutions worse than the cutoff", solver::SUBA_CUT_REJECTS.load(std::sync::atomic::Ordering::Relaxed));
    }
    if crate::simplex::nb_diag::on() {
        crate::simplex::nb_diag::print(t_solve.elapsed().as_secs_f64());
    }
    crate::simplex::mip_lp::set_cold_race_threads(prev_race);
    crate::simplex::mip_lp::set_seq_ipm(prev_seq);
    solver::MIP_THREADS_DEFAULT.store(prev_mt, std::sync::atomic::Ordering::Relaxed);
    // (サブ MIP の終わりでは出さない: 同じスレッドの外側の計時の途中の値になる。analysis/proofprop_20261123)
    if env_str!("ENOMOTO_MIP_XPROF").is_some() && !params.submip {
        crate::simplex::slope_intercept_dual::xprof("tail");
        // どのスレッドの計時か (スレッドごとに貯めるので、同じスレッドで 2 回出れば後の方が全部を含む)
        eprintln!("XPROFBLOCK {:?}", std::thread::current().id());
        for (l, ns) in crate::simplex::slope_intercept_dual::xprof_take() {
            eprintln!("XPROF {l:10} {:10.1} ms", ns as f64 / 1e6);
        }
        // 木を分け合う従のスレッドの合計 (analysis/proofprop_20261123)
        for (l, ns) in crate::simplex::slope_intercept_dual::xprof_workers_take() {
            eprintln!("XPROFW {l:10} {:10.1} ms", ns as f64 / 1e6);
        }
    }
    // 診断 `ENOMOTO_DEBUG_TICKSUM` (analysis/lpgap_20261126 の 9.8 節): このスレッドの分解の数と ticks の部分ごとの数 (lpit は LP の反復) と重み付きの合計
    if env_str!("ENOMOTO_DEBUG_TICKSUM").is_some() && !params.submip {
        let raw = crate::ticks::raw();
        let parts: Vec<String> = (0..crate::ticks::NK).map(|k| format!("{}={}", crate::ticks::NAMES[k], raw[k])).collect();
        eprintln!("TICKSUM builds={} total={:.4e} {}", crate::simplex::lu::LU_BUILDS.with(|c| c.get()), crate::ticks::total(), parts.join(" "));
    }
    let s = p.sense_sign;
    let mut r = r;
    // 返す解を研磨し、目的値を誤差補償つきの和で計算し直す ([`polish_solution`])
    if r.status != MipStatus::Unbounded {
        if let Some(x) = r.x.as_mut() {
            // `ENOMOTO_MIP_LATE_NO_RESOLVE`: 時間を使い切っていれば研磨の LP は解かない (整数の丸めと目的値の計算し直しだけ)。
            // 研磨は元の問題の前処理 + LP で 0.2-0.35 秒かかり、時間の上限の超過の残りの大半だった (uccase12・physiciansched6-2)
            // `ENOMOTO_MIP_POLISH_STRICT` (2026-11-25 から既定。analysis/nattai_20261125/): 時間を使い切っていても研磨し、研磨の LP に期限を渡さない。
            // 時間切れの回の暫定解は研磨されず、B&B の LP の許容誤差の範囲の違反 (行ごとには 1e-6 以下) が積もって、目的値が本当の値より
            // 低いまま返っていた (neos-5188808-nattai: 0.12230 と返した解の整数を固定した LP は 0.12240、既知の最適値 0.110284 より良い 0.110186 も返した)
            // `ENOMOTO_MIP_POLISH_WARM`: 返す解は木の求解の終わりに問題の空間で研磨してあるので、ここは既定と同じ扱い (時間を使い切って
            // いれば解かない・期限を渡す) にし、取り替えなければ木の求解の終わりに置いた漏れの見積り ([`WARM_LEAK`]) を足す
            let warm = polish_strict() && polish_warm_mode() > 0;
            let strict = polish_strict() && !warm;
            let late = !strict && env_def!("ENOMOTO_MIP_LATE_NO_RESOLVE", "1").is_some() && t_solve.elapsed().as_secs_f64() >= params.time_limit;
            // `ENOMOTO_MIP_POLISH_DEADLINE` (既定 1、`=0` で以前の形 = 期限なし): 研磨の LP に求解の期限 (始め + 時間の上限) を渡し、
            // 間に合わなければ研磨しない (整数の丸めと目的値の計算し直しだけ)。dano3_3 では MIP が 50〜52 秒で最適で終わった後、整数を固定した
            // LP を基底なしの単体法で解くのに 16〜17 秒かかり、60 秒の上限を越えて 66〜69 秒で返っていた (analysis/ipmx_harm_20261106.md)
            let dl = (!strict && env_def!("ENOMOTO_MIP_POLISH_DEADLINE", "1").is_some() && params.time_limit.is_finite())
                .then(|| t_solve + std::time::Duration::from_secs_f64(params.time_limit.clamp(0.0, 1e9)));
            let x0 = warm.then(|| x.clone());
            let mut z = crate::cancel::with_deadline(dl, || polish_solution(variables, objective, constraints, &p, x, !late));
            if let Some(x0) = x0 {
                use std::sync::atomic::Ordering;
                let leak = f64::from_bits(WARM_LEAK.load(Ordering::Relaxed));
                // 研磨が整数の丸めだけ (連続列が変わらない) なら、漏れは残っている
                let same = (0..p.n).all(|j| p.is_int[j] || x[j] == x0[j]);
                if same && leak > 0.0 {
                    z += leak;
                }
                if solver::end_diag() || params.verbose || env_str!("ENOMOTO_MIP_POLISH_STATS").is_some() {
                    eprintln!(
                        "MIP: POLISH_WARM: LPs {} (replaced {}, not solved {}), leak estimates {}, {:.3}s, {} iters; uncertain estimates in the tree {}; uncertain at the end {} (returned the verified solution {}, polished without the deadline {} in {:.3}s); final leak estimate {leak:.3e} ({})",
                        WARM_POLISH[0].load(Ordering::Relaxed),
                        WARM_POLISH[1].load(Ordering::Relaxed),
                        WARM_POLISH[2].load(Ordering::Relaxed),
                        WARM_POLISH[3].load(Ordering::Relaxed),
                        WARM_POLISH[4].load(Ordering::Relaxed) as f64 / 1e9,
                        WARM_POLISH[5].load(Ordering::Relaxed),
                        WARM_POLISH[6].load(Ordering::Relaxed),
                        WARM_POLISH[7].load(Ordering::Relaxed),
                        WARM_POLISH[8].load(Ordering::Relaxed),
                        WARM_POLISH[9].load(Ordering::Relaxed),
                        WARM_POLISH[10].load(Ordering::Relaxed) as f64 / 1e9,
                        if same { "added" } else { "not added: polished" }
                    );
                }
            }
            if solver::end_diag() || params.verbose {
                eprintln!("MIP: polished the solution ({:.2}s)", t_solve.elapsed().as_secs_f64());
            }
            if polish_strict_mode() >= 2 && (solver::end_diag() || params.verbose || env_str!("ENOMOTO_MIP_POLISH_STATS").is_some()) {
                use std::sync::atomic::Ordering;
                eprintln!(
                    "MIP: POLISH_STRICT=2: incumbent polishes {} (replaced {}), {:.3}s",
                    INC_POLISH[0].load(Ordering::Relaxed),
                    INC_POLISH[1].load(Ordering::Relaxed),
                    INC_POLISH[2].load(Ordering::Relaxed) as f64 / 1e9
                );
            }
            if r.status == MipStatus::Optimal {
                r.best_bound = z;
            } else {
                r.best_bound = r.best_bound.min(z);
            }
            r.objective = Some(z);
        }
    }
    // `ENOMOTO_MIP_BOUND_STEP` (試作、既定では使わない): 元の問題の目的値が刻みの格子 (`offset + k * step`) にしか乗らなければ、
    // 報告の下界を格子の点に切り上げる (木の中の切り上げ [`solver::Solver::step_lb`] は前処理後の問題の刻みで行う。こちらは
    // 前処理・後処理の修復・再スタートを通った後の値に、元の問題で確かな刻みを当てる)。切り上げた下界が暫定解の値に届けば最適
    if solver::bound_step_on() && !params.submip && matches!(r.status, MipStatus::TimeLimit | MipStatus::NodeLimit | MipStatus::NotSolved) {
        // 試作 `ENOMOTO_MIP_OBJINT` の `c`: 元の問題でも、HO2 の暗黙の整数・固定の列・有界な連続部の範囲 ([`solver::ObjLat`]) で切り上げる
        let lat = if solver::objint('c') { solver::ObjLat::of(&p, &p.implied_integers_ho2(), &p.col_lo, &p.col_up) } else { None };
        let st = match lat {
            Some(l) => Some((l.s, l.round_up(r.best_bound))),
            None => p.objective_step().map(|step| (step, solver::round_lb_to_step(r.best_bound, step, p.offset))),
        };
        if let Some((step, lb)) = st {
            match r.objective {
                Some(z) => {
                    r.best_bound = lb.min(z).max(r.best_bound.min(z));
                    if lb >= z && r.status != MipStatus::NotSolved {
                        if params.verbose {
                            eprintln!("MIP: the lower bound {:.10e} rounded up by the objective step {step} reaches the incumbent {z:.10e}; optimal", r.best_bound);
                        }
                        r.status = MipStatus::Optimal;
                        r.best_bound = z;
                    }
                }
                None => r.best_bound = lb,
            }
        }
    }
    // 非有界なら目的値・解は返さない (見つけた実行可能解は最適ではない)
    let unbounded = r.status == MipStatus::Unbounded;
    let objective_value = if unbounded { None } else { r.objective.map(|z| s * z) };
    let gap = match r.objective {
        Some(z) if r.best_bound.is_finite() => ((z - r.best_bound).max(0.0)) / z.abs().max(1.0),
        Some(_) if r.status == MipStatus::Optimal => 0.0,
        _ => f64::INFINITY,
    };
    let (first_sol, root_bound) = RUN_STATS.lock().map(|st| (st.first_sol, st.root_bound)).unwrap_or((None, None));
    let summary = MipSummary {
        best_bound: s * r.best_bound,
        gap,
        nodes: r.nodes,
        lp_iterations: r.lp_iterations,
        root_bound: root_bound.map_or(f64::NAN, |z| s * z),
        // 打ち切り値より良くない最初の解は数えない (返す解と揃える、[`patch_user_cutoff`])
        first_solution_time: first_sol.filter(|&(_, z)| params.submip || !at_or_above(z, params.cutoff)).map_or(f64::NAN, |(t, _)| t),
        first_solution_objective: first_sol.filter(|&(_, z)| params.submip || !at_or_above(z, params.cutoff)).map_or(f64::NAN, |(_, z)| s * z),
    };
    let status = match r.status {
        MipStatus::Optimal => Status::Optimal,
        MipStatus::Infeasible => Status::Infeasible,
        MipStatus::Unbounded => Status::Unbounded,
        MipStatus::InfeasibleOrUnbounded => Status::InfeasibleOrUnbounded,
        MipStatus::TimeLimit => Status::TimeLimit,
        MipStatus::NodeLimit => Status::NodeLimit,
        MipStatus::NotSolved => Status::NotSolved,
    };
    SolveResult { status, objective: objective_value, x: if unbounded { None } else { r.x }, node_limit_hit: r.status == MipStatus::NodeLimit, mip: Some(summary) }
}

/// 誤差補償つき (Neumaier) の和。
fn exact_sum(it: impl Iterator<Item = f64>) -> f64 {
    let (mut sum, mut comp) = (0.0f64, 0.0f64);
    for v in it {
        let t = sum + v;
        comp += if sum.abs() >= v.abs() { (sum - t) + v } else { (v - t) + sum };
        sum = t;
    }
    sum + comp
}

/// 元の問題での点 `x` の境界・行の違反の最大値 (行の活動量は誤差補償つきの和)。
fn max_violation(p: &MipProblem, x: &[f64]) -> f64 {
    // NaN の違反は無限とする (`f64::max` は NaN を捨てるので、そのままでは NaN を含む点の違反が 0 になり、研磨の採否で通ってしまう)
    let upd = |v: f64, d: f64| if d.is_nan() { f64::INFINITY } else { v.max(d) };
    let mut v = 0.0f64;
    for j in 0..p.n {
        v = upd(upd(v, p.col_lo[j] - x[j]), x[j] - p.col_up[j]);
    }
    for (i, row) in p.rows.iter().enumerate() {
        let a = exact_sum(row.iter().map(|&(j, c)| c * x[j]));
        v = upd(upd(v, p.row_lo[i] - a), a - p.row_up[i]);
    }
    v
}

/// 最小化形の目的値 (定数項込み) を誤差補償つきの和で計算する。
fn exact_objective(p: &MipProblem, x: &[f64]) -> f64 {
    exact_sum(std::iter::once(p.offset).chain((0..p.n).map(|j| p.cost[j] * x[j])))
}

/// 返す解の研磨: 整数変数をちょうどの整数に丸め、連続変数があれば整数変数をその値に固定した元の問題の LP を
/// 解き直して連続変数の値を取り替える。分枝限定法の中の LP は前処理・スケーリング・境界の摂動・許容誤差の
/// 範囲の解を返すので、そのままでは目的値に 1e-12〜1e-9 程度の相対誤差が残る (khb05250, binkar10_1)。
/// 解き直した解は、元の問題の違反 (境界・行) が増えず目的値が悪くならないとき、または違反が真に減って目的値の悪化が
/// 相対 1e-6 以下のときに採る。最小化形の目的値を返す
/// (誤差補償つきの和)。`ENOMOTO_MIP_NO_POLISH` なら丸めと目的値の計算し直しだけ。
/// `ENOMOTO_MIP_POLISH_STRICT` (analysis/nattai_20261125/) の段階: 0 = 使わない、
/// 1 = [`polish_solution`] を時間切れの回でも回し、違反が減る解き直しは目的値が悪くなっても採る、
/// 2 = 1 に加えて、暫定解を受け入れるたびに [`polish_in_problem`] で研磨し、打ち切り値・共有する最良値も研磨した値にする
/// (`ENOMOTO_MIP_POLISH_WARM` の既定では [`polish_in_problem_warm`] で温めて研磨する)。
/// **2026-11-25 に既定にした: 値 `2` (`ENOMOTO_MIP_POLISH_WARM=d` と組)。`0`・`off` で前の動き (使わない)。**
/// 根拠: analysis/polishp_20261125/ の P 58 問 × 種 1〜3 × 1・4 スレッド (1044 回、README の「P 全体での WARM=d の比べ」):
/// 1 スレッド 0.998 [0.989, 1.006]・解けた 118→123、4 スレッド 0.995 [0.979, 1.012]・139→139、食い違い 0、60.5 秒の超過 0。
/// 返す目的値が許容誤差の漏れで本当の値より低くなる問題 (neos-5188808-nattai で 1e-4) を正す
pub(super) fn polish_strict_mode() -> u8 {
    static V: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
    *V.get_or_init(|| match env_def!("ENOMOTO_MIP_POLISH_STRICT", "2") {
        Some(v) if v == "2" => 2,
        Some(_) => 1,
        None => 0,
    })
}

pub(super) fn polish_strict() -> bool {
    polish_strict_mode() >= 1
}

/// `ENOMOTO_MIP_POLISH_STRICT=2` の集計: 研磨した回数、解を取り替えた回数、かかった時間 (ナノ秒)
pub(super) static INC_POLISH: [std::sync::atomic::AtomicU64; 3] = [const { std::sync::atomic::AtomicU64::new(0) }; 3];

/// 範囲つきの行 `lo <= a x <= up` の並びを、LP の入口 (`solve_lp` など) の行 ([`ConstraintRow`]) に直す: `lo == up` なら等式 1 本、
/// そうでなければ有限な側ごとに `>=`・`<=` (この順) の 2 本まで。`cap` は行の数の見込み。研磨・後処理の修復・前処理の代わりの道・
/// 根の LP の前処理つきの解き直しの共通部分 (analysis/unify_20261126 の 1 節の 12)
pub(crate) fn ranged_constraints<'r>(rows: impl Iterator<Item = (&'r [(usize, f64)], f64, f64)>, cap: usize) -> Vec<ConstraintRow> {
    use crate::types::{LinearExpr, RowSense};
    let mut cons = Vec::with_capacity(cap);
    for (row, lo, up) in rows {
        let expr = LinearExpr { coeffs: row.iter().copied().collect(), constant: 0.0 };
        if lo == up {
            cons.push(ConstraintRow { expr, sense: RowSense::Eq, rhs: lo });
            continue;
        }
        if lo.is_finite() {
            cons.push(ConstraintRow { expr: expr.clone(), sense: RowSense::Ge, rhs: lo });
        }
        if up.is_finite() {
            cons.push(ConstraintRow { expr, sense: RowSense::Le, rhs: up });
        }
    }
    cons
}

/// 問題 `p` の行を [`ranged_constraints`] で直したもの
fn problem_constraints(p: &MipProblem) -> Vec<ConstraintRow> {
    ranged_constraints((0..p.m).map(|i| (&p.rows[i][..], p.row_lo[i], p.row_up[i])), p.m)
}

/// 費用 `cost` (0 の列は除く) と定数 `constant` の最小化の目的
pub(crate) fn min_objective(cost: &[f64], constant: f64) -> Objective {
    use crate::types::{LinearExpr, Sense};
    Objective { expr: LinearExpr { coeffs: (0..cost.len()).filter(|&j| cost[j] != 0.0).map(|j| (j, cost[j])).collect(), constant }, sense: Sense::Minimize }
}

/// `ENOMOTO_MIP_POLISH_STRICT=2` (試作): 暫定解 `x` (問題 `p` の空間、目的値 `z`) の整数列を固定した `p` を、B&B の LP とは別に
/// 一から (前処理 + 単体法) 解き直す。B&B の LP はカット・境界のずらしの入った問題を許容誤差の範囲で解くので、行ごとには許容誤差以下の違反が
/// 多くの行で積もり、目的値が本当の値より低い暫定解になる (neos-5188808-nattai: 112 行 × 9.5e-7 で 1e-4)。採り方は [`polish_solution`] の
/// 試作と同じ (違反が増えず目的値が悪くならない、または違反が真に減る)。取り替えたら新しい目的値を返す
pub(super) fn polish_in_problem(p: &MipProblem, x: &mut Vec<f64>, z: f64) -> Option<f64> {
    use std::sync::atomic::Ordering;
    if p.is_int.iter().all(|&b| b) {
        return None;
    }
    let t = std::time::Instant::now();
    let vars: Vec<VariableData> = (0..p.n)
        .map(|j| {
            let (lb, ub) = if p.is_int[j] { (x[j].round(), x[j].round()) } else { (p.col_lo[j], p.col_up[j]) };
            VariableData { vtype: VarType::Continuous, lb, ub }
        })
        .collect();
    let objective = min_objective(&p.cost, p.offset);
    let cons = problem_constraints(p);
    let r = solve_lp(&vars, &objective, &cons, RootSolver::Simplex, LpOptions::default());
    INC_POLISH[0].fetch_add(1, Ordering::Relaxed);
    let mut out = None;
    if let (Status::Optimal, Some(mut xn)) = (r.status, r.x) {
        for j in 0..p.n {
            if p.is_int[j] {
                xn[j] = x[j].round();
            }
        }
        let zn = exact_objective(p, &xn);
        let (vo, vn) = (max_violation(p, x), max_violation(p, &xn));
        let scale = z.abs().max(1.0);
        if (vn <= vo.max(1e-9) && zn <= z + 1e-9 * scale) || vn < vo {
            *x = xn;
            out = Some(zn);
            INC_POLISH[1].fetch_add(1, Ordering::Relaxed);
        }
    }
    INC_POLISH[2].fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
    out
}

/// 試作 `ENOMOTO_MIP_POLISH_PRESOLVE` (analysis/overrun240_20261127、既定では使わない): 最後の研磨の LP (整数を固定した `p`) を、温めた単体法の代わりに
/// LP の前処理 + 単体法で解く ([`polish_in_problem`] と同じ解き方、[`polish_presolved`])。`1` = 最適で終わらないとき (時間切れなど。W を使う最適の判定に関わらない) だけ、`a` = いつも。
/// 整数を固定すると多くの行が 1 列の境界になる問題 (thor50dday・neos-873061・cuanza・kasavu) で、研磨の LP が前処理で大きく縮む (README 4.3 節)。
/// この経路は双対値を返さないので、許容の漏れの上界 W ([`POLISH_TOL_LEAK`]) は 0 にする (最適の判定が厳しい側になるだけ)
pub(super) fn polish_presolve_mode() -> Option<char> {
    static V: std::sync::OnceLock<Option<char>> = std::sync::OnceLock::new();
    *V.get_or_init(|| match env_str!("ENOMOTO_MIP_POLISH_PRESOLVE") {
        Some(v) if v.contains('a') => Some('a'),
        Some(v) if v != "0" && v != "off" => Some('1'),
        _ => None,
    })
}

/// 試作 `ENOMOTO_MIP_POLISH_PRESOLVE`: 暫定解 `x` (問題 `p` の空間、目的値 `z`) の整数列を固定した `p` を LP の前処理 + 単体法で解き、
/// 採り方は [`polish_in_problem_warm`] と同じ (違反が増えず目的値が悪くならない、または違反が真に減る)。期限 `deadline` で止める。
/// 解けなければ `NotSolved` (呼び出し側は温めた単体法に回す)。元の空間への戻し (前処理の後始末) の誤りは、戻した解の違反を元の問題で測る
/// 採り方の判定で捨てられる
pub(super) fn polish_presolved(p: &MipProblem, x: &mut Vec<f64>, z: f64, deadline: Option<std::time::Instant>) -> WarmPolish {
    if p.is_int.iter().all(|&b| b) {
        return WarmPolish::Kept;
    }
    if deadline.is_some_and(|d| std::time::Instant::now() >= d) {
        return WarmPolish::NotSolved;
    }
    let t = std::time::Instant::now();
    let vars: Vec<VariableData> = (0..p.n)
        .map(|j| {
            let (lb, ub) = if p.is_int[j] { (x[j].round(), x[j].round()) } else { (p.col_lo[j], p.col_up[j]) };
            VariableData { vtype: VarType::Continuous, lb, ub }
        })
        .collect();
    let objective = min_objective(&p.cost, p.offset);
    let cons = problem_constraints(p);
    let tb = t.elapsed().as_secs_f64();
    let r = crate::ticks::without_budget(|| crate::cancel::with_deadline(deadline, || solve_lp(&vars, &objective, &cons, RootSolver::Simplex, LpOptions::default())));
    POLISH_TOL_LEAK.with(|c| c.set((0.0, 0.0)));
    let mut out = WarmPolish::NotSolved;
    let mut info = String::new();
    let st_dbg = format!("{:?}", r.status);
    if let (Status::Optimal, Some(mut xn)) = (r.status, r.x) {
        out = WarmPolish::Kept;
        for j in 0..p.n {
            if p.is_int[j] {
                xn[j] = x[j].round();
            }
        }
        let zn = exact_objective(p, &xn);
        let (vo, vn) = (max_violation(p, x), max_violation(p, &xn));
        let scale = z.abs().max(1.0);
        info = format!(" z {z:.10e} -> {zn:.10e} viol {vo:.3e} -> {vn:.3e}");
        if (vn <= vo.max(1e-9) && zn <= z + 1e-9 * scale) || vn < vo {
            *x = xn;
            out = WarmPolish::Replaced(zn);
        }
    }
    if env_str!("ENOMOTO_MIP_POLISH_STATS").is_some_and(|v| v.contains('v') || v.contains('r')) {
        eprintln!("MIP: POLISH_PRESOLVE: {st_dbg} {:.3}s (building {tb:.3}s){info} -> {}", t.elapsed().as_secs_f64(), match out {
            WarmPolish::Replaced(_) => "replaced",
            WarmPolish::Kept => "kept",
            WarmPolish::NotSolved => "not solved",
        });
    }
    out
}

/// `ENOMOTO_MIP_POLISH_WARM` (`ENOMOTO_MIP_POLISH_STRICT` と組で使う。analysis/nattai_20261125/。
/// **2026-11-25 に既定にした: 値 `d`。`0`・`off` で使わない (研磨の LP を前処理つきで一から解き、期限を渡さない前の形)。**
/// **2026-11-26 から、付けなければ [`polish_final_on`] (既定) で `e`。**
/// 根拠は [`polish_strict_mode`] と同じ。dano3_3・dano3_5 の時間切れの回で上限を越えない (60.02 秒。期限なしでは 73〜78 秒)):
/// 0 = 使わない、1 = 研磨の LP を B&B の LP の基底から温めて解く (返す前の研磨には期限を渡さない)、
/// 2 (`d`) = 1 に加えて返す前の研磨にも期限を渡し、間に合わなければ漏れの見積り ([`solver::Solver::leak_estimate`]) で目的値を正す、
/// 3 (`e`) = 研磨の LP を解かず、いつも漏れの見積りで目的値を正す。
/// 木の中の研磨 (`ENOMOTO_MIP_POLISH_STRICT=2`) には、どれも求解の期限を渡し、間に合わなければ研磨せずに受け取って見積りを足し、
/// 「未研磨」の印をつける。返す前の研磨は、木の中で研磨が済んだ暫定解なら飛ばす。
/// 見積りの双対値と温める基底は、暫定解の連続部を出した LP ([`solver::Solver::resolve_continuous`] の整数を固定した LP、
/// または双対値で最適と示せた今の LP) のもの
/// `ENOMOTO_MIP_POLISH_WARM` を付けていなければ、[`polish_final_on`] (既定) で 3 (`e`)、切っていれば 2 (`d`)。
pub(super) fn polish_warm_mode() -> u8 {
    static V: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
    *V.get_or_init(|| {
        if env_str!("ENOMOTO_MIP_POLISH_WARM").is_none() && polish_final_on() {
            return 3;
        }
        match env_def!("ENOMOTO_MIP_POLISH_WARM", "d") {
            Some(v) if v == "e" => 3,
            Some(v) if v == "d" => 2,
            Some(_) => 1,
            None => 0,
        }
    })
}

/// `ENOMOTO_MIP_POLISH_FINAL` (**2026-11-26 に既定にした。`0`・`off` で前の形 `POLISH_WARM=d` に戻す**。analysis/rootpar_ph_20261126 の 5 節):
/// 研磨は返す前の 1 回だけにする。木の中では暫定解を研磨せず、漏れの見積りを足して受け取る (`POLISH_WARM=e`)。返す前に研磨の LP を期限なしで
/// 1 回解き (試作 `ROOTPAR_PH` の `g`)、空いたスレッドがあれば温めた単体法と内点法 + クロスオーバーの同時実行で解く (`j`)。
/// 最後の研磨で上がった幅が刈りの余裕と W_mip (研磨の LP の双対値 × MIP の実行可能性の許容) の大きい方を越えれば最適と言わない
/// (「最適」は MIP の実行可能性の許容の中での最適。以前の `d` は単体法の主の許容の水準)。dano3_3 (4 スレッド) で木の中の研磨 22〜33 秒が
/// 最後の 1 回 3.3 秒になる。`ENOMOTO_MIP_POLISH_WARM` を付ければその形 (`g`・`j` は `ROOTPAR_PH` で付けたときだけ)
pub(super) fn polish_final_on() -> bool {
    env_def!("ENOMOTO_MIP_POLISH_FINAL", "1").is_some()
}

/// `ENOMOTO_MIP_POLISH_WARM` で、返す暫定解の目的値に足す漏れの見積り (f64 のビット)。木の求解の終わり ([`solver::Solver::finish`]) で
/// 主のスレッドが置き、返す前の研磨が解き直した解に取り替えなかったときに足す
pub(super) static WARM_LEAK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// `ENOMOTO_MIP_POLISH_WARM` の集計: 研磨の LP を解いた回数、取り替えた回数、間に合わなかった回数、見積りで正した回数、
/// かかった時間 (ナノ秒)、単体法の反復の数、
/// 木の中で見積りが当てにならなかった回数 ([`solver::WarmMark::Unknown`])、返す解の値が確かでないまま終わった回数、
/// そのとき研磨で確かめた解に戻した回数、期限なしで研磨の LP を解いた回数、その時間 (ナノ秒) (analysis/nattai_20261125 の 10 節)
pub(super) static WARM_POLISH: [std::sync::atomic::AtomicU64; 11] = [const { std::sync::atomic::AtomicU64::new(0) }; 11];

/// `ENOMOTO_MIP_POLISH_WARM` の研磨の結果
pub(super) enum WarmPolish {
    /// 解き直した解に取り替えた (新しい目的値)
    Replaced(f64),
    /// 解き直したが、取り替えなかった
    Kept,
    /// 期限までに解けなかった、または解かなかった
    NotSolved,
}

/// `ENOMOTO_MIP_POLISH_WARM=1` (試作): 暫定解 `x` (問題 `p` の空間、目的値 `z`) の整数列を固定した `p` を、B&B の LP の基底 `basis`
/// (カットの行の分は捨てる) から温めて単体法で解く。前処理はしない。`deadline` を過ぎたら打ち切る。採り方は [`polish_in_problem`] と同じ
pub(super) fn polish_in_problem_warm(p: &MipProblem, x: &mut Vec<f64>, z: f64, basis: Option<&lp::Basis>, deadline: Option<std::time::Instant>) -> WarmPolish {
    polish_in_problem_warm_ext(p, x, z, basis, deadline, false)
}

thread_local! {
    /// 試作 `ROOTPAR_PH` の `f`・`g` (README 5.7 節): このスレッドで最後に解けた研磨の LP の双対値で測った、許容の中で実行可能な点が持ちうる漏れの最大
    /// W(ε) = Σ_行 |y_i| ε(b_i) + Σ_連続列 |d_j| ε(境界)。(W_lp, W_mip) の 2 つ: W_lp は ε = 1e-9 max(1, |b|) (単体法の主の許容。研磨した解 = `d` の水準)、
    /// W_mip は ε = 1e-6 + 1e-9 |b| (MIP の実行可能性の許容 [`domain::FEASTOL`]、`is_feasible` と同じ。木が暫定解を受け取る水準)
    pub(super) static POLISH_TOL_LEAK: std::cell::Cell<(f64, f64)> = const { std::cell::Cell::new((0.0, 0.0)) };
}

/// 研磨の LP の双対値から [`POLISH_TOL_LEAK`] の (W_lp, W_mip) を計算する
fn tol_leak_bound(p: &MipProblem, lp: &crate::simplex::mip_lp::TwoStageLp) -> (f64, f64) {
    let b = |a: f64, c: f64| [a, c].iter().filter(|v| v.is_finite()).fold(0.0f64, |m, v| m.max(v.abs()));
    let e_lp = |bb: f64| 1e-9 * bb.max(1.0);
    let e_mip = |bb: f64| domain::FEASTOL + 1e-9 * bb;
    let y = lp.row_duals();
    let d = lp.reduced_costs();
    let (mut w1, mut w2) = (0.0, 0.0);
    for i in 0..p.m.min(y.len()) {
        let bb = b(p.row_lo[i], p.row_up[i]);
        w1 += y[i].abs() * e_lp(bb);
        w2 += y[i].abs() * e_mip(bb);
    }
    for j in 0..p.n.min(d.len()) {
        if !p.is_int[j] {
            let bb = b(p.col_lo[j], p.col_up[j]);
            w1 += d[j].abs() * e_lp(bb);
            w2 += d[j].abs() * e_mip(bb);
        }
    }
    (w1, w2)
}

/// 研磨の LP の反復の上限 (試作 `ROOTPAR_PH` の `j`): `ENOMOTO_T_MIP_G5SEQ_C` (0.5) × (行 + 列)。1 スレッドの基底のない求解の
/// 二段解法 → 内点法 + クロスオーバーの切り替え (`G5SEQ`) と同じ物差し
pub(super) fn polish_iter_cap(rows: usize, cols: usize) -> u64 {
    (tunable!("ENOMOTO_T_MIP_G5SEQ_C", 0.5, f64) * (rows + cols) as f64).max(100.0) as u64
}

/// [`polish_in_problem_warm`] の本体。`cold` なら基底を使わず、基底のない求解の既存の入口 (`solve_cold_entry`: 空いたスレッドがあれば同時実行、
/// なければ二段解法 → 内点法 + クロスオーバー) で解く。試作 `ROOTPAR_PH` の `j` ([`solver::rootpar_ph`]) では、温めた単体法に反復の上限
/// [`polish_iter_cap`] を付け、上限に当たれば基底を捨てて同じ入口で解き直す (基底が当てにならない扱い)。このスレッドに同時実行のスレッドが
/// 2 本以上あれば、上限を付けずに温めた単体法と内点法 + クロスオーバーを同時に解く (`solve_race_warm`)
pub(super) fn polish_in_problem_warm_ext(p: &MipProblem, x: &mut Vec<f64>, z: f64, basis: Option<&lp::Basis>, deadline: Option<std::time::Instant>, cold: bool) -> WarmPolish {
    use std::sync::atomic::Ordering;
    if p.is_int.iter().all(|&b| b) {
        return WarmPolish::Kept;
    }
    if deadline.is_some_and(|d| std::time::Instant::now() >= d) {
        return WarmPolish::NotSolved;
    }
    let t = std::time::Instant::now();
    let mut lo = p.col_lo.clone();
    let mut up = p.col_up.clone();
    for j in 0..p.n {
        if p.is_int[j] {
            let v = x[j].round();
            lo[j] = v;
            up[j] = v;
        }
    }
    let mut lp = crate::simplex::mip_lp::TwoStageLp::new(&lo, &up, &p.cost, &p.rows, &p.row_lo, &p.row_up);
    // 基底の写し: 固定した整数列は基底から外し (非基底の下限)、カットの行の分は捨てる (行の数の過不足は `set_basis` の修復でスラックを足す・構造列を外す)
    let mut info = (0usize, 0usize, 0usize);
    // 診断 `ENOMOTO_MIP_POLISH_STATS` の `c`: 基底を写さずに解く (温める効き目の比べ)
    let basis = basis.filter(|_| !cold && !env_str!("ENOMOTO_MIP_POLISH_STATS").is_some_and(|v| v.contains('c')));
    // 診断 `ENOMOTO_MIP_POLISH_STATS` の `e` (analysis/overrun240_20261127): 基底を捨てて基底のない求解の入口 (`solve_cold_entry`) で解く
    let cold = cold || env_str!("ENOMOTO_MIP_POLISH_STATS").is_some_and(|v| v.contains('e'));
    let basis = basis.filter(|_| !cold);
    if let Some(b) = basis {
        let mut b = b.clone();
        for j in 0..p.n.min(b.col.len()) {
            if p.is_int[j] && b.col[j] == lp::VarStatus::Basic {
                b.col[j] = lp::VarStatus::Lower;
                info.0 += 1;
            }
        }
        info.1 = b.row.len().saturating_sub(p.m);
        b.row.truncate(p.m);
        info.2 = b.col.iter().filter(|&&s| s == lp::VarStatus::Basic).count() + b.row.iter().filter(|&&s| s == lp::VarStatus::Basic).count();
        lp.set_basis(&b);
        // 診断 `ENOMOTO_MIP_POLISH_STATS` の `b` (analysis/overrun240_20261127): 写した基底 (修復の後) の出発の点の主の実行不能 (数と相対の量の和)
        if env_str!("ENOMOTO_MIP_POLISH_STATS").is_some_and(|v| v.contains('b')) {
            let tb = std::time::Instant::now();
            let si = lp.basis_start_info(&b.col);
            eprintln!("MIP: POLISH_STATS b: start basis primal infeasible {:?} (count, relative sum), {:.3}s", si, tb.elapsed().as_secs_f64());
        }
    }
    // 診断 `ENOMOTO_MIP_POLISH_STATS` の `s`: 研磨の LP の大きさ (行・列・非零、整数を固定した列、固定されない連続列)
    if env_str!("ENOMOTO_MIP_POLISH_STATS").is_some_and(|v| v.contains('s')) {
        let nnz: usize = p.rows.iter().map(|r| r.len()).sum();
        let nint = p.is_int.iter().filter(|&&b| b).count();
        let ncont_free = (0..p.n).filter(|&j| !p.is_int[j] && p.col_lo[j] < p.col_up[j]).count();
        // 整数を固定した後の行ごとの自由な連続列の数 (0 = 定数の行、1 = 列の境界になる行。前処理で消せる行の目安)
        let mut k0 = 0usize;
        let mut k1 = 0usize;
        for r in &p.rows {
            let c = r.iter().filter(|&&(j, _)| !p.is_int[j] && p.col_lo[j] < p.col_up[j]).count();
            if c == 0 {
                k0 += 1;
            } else if c == 1 {
                k1 += 1;
            }
        }
        // 前処理の見積り: 整数を固定し、自由な連続列が 1 本の行を列の境界にして外し、境界が閉じた列を固定することを繰り返して残る行の数
        let mut fixed: Vec<bool> = (0..p.n).map(|j| p.is_int[j] || p.col_lo[j] >= p.col_up[j]).collect();
        let (mut clo, mut cup) = (p.col_lo.clone(), p.col_up.clone());
        let mut alive = vec![true; p.m];
        let mut passes = 0;
        loop {
            passes += 1;
            let mut changed = false;
            for (i, r) in p.rows.iter().enumerate() {
                if !alive[i] {
                    continue;
                }
                let free: Vec<&(usize, f64)> = r.iter().filter(|&&(j, _)| !fixed[j]).collect();
                if free.len() > 1 {
                    continue;
                }
                alive[i] = false;
                changed = true;
                if let [&(j, a)] = free[..] {
                    let rest: f64 = r.iter().filter(|&&(k, _)| k != j).map(|&(k, b)| b * if p.is_int[k] { x[k].round() } else { clo[k] }).sum();
                    let (l, u) = ((p.row_lo[i] - rest) / a, (p.row_up[i] - rest) / a);
                    let (l, u) = if a > 0.0 { (l, u) } else { (u, l) };
                    clo[j] = clo[j].max(l);
                    cup[j] = cup[j].min(u);
                    if clo[j] >= cup[j] - 1e-12 {
                        fixed[j] = true;
                    }
                }
            }
            if !changed || passes >= 20 {
                break;
            }
        }
        let left = alive.iter().filter(|&&a| a).count();
        let cols_left = (0..p.n).filter(|&j| !fixed[j]).count();
        eprintln!(
            "MIP: POLISH_STATS s: rows {} cols {} nonzeros {} integer (fixed) {} continuous not fixed {}; rows with 0 free continuous columns {} with 1 {}; after singleton-row reductions ({passes} passes) rows left {left} free columns left {cols_left}",
            p.m, p.n, nnz, nint, ncont_free, k0, k1
        );
    }
    // 呼び出し元の手間の期限 ([`crate::ticks::push_budget`]) は外し、求解の期限 (壁時計) だけで止める。ヒューリスティクスの中で受け取った
    // 解の研磨が手間の期限で 3 反復で止まり、研磨しないまま見積り (その解を出した LP の双対値がなく、今の LP の双対値から取った小さな値) で
    // 受け取って、4 スレッドの nattai が最適値より 9.4e-5 良い値を返した (analysis/nattai_20261125 の 9 節)
    let lim = lp::SolveLimits { deadline, ..Default::default() };
    let jcap = solver::polish_final('j');
    let mut how = "";
    let st = crate::ticks::without_budget(|| {
        if cold {
            how = " (cold entry)";
            lp.solve_cold_entry(&lim)
        } else if jcap && basis.is_some() && crate::simplex::mip_lp::cold_race_threads() >= 2 && crate::cancel::current().is_none() {
            // 空いたスレッドがあれば (外側の根・求解の終わりの主のスレッド)、温めた単体法と内点法 + クロスオーバーを最初から同時に解く
            // (既存の `solve_race_warm`。上限まで温めた側だけで待つ分がない。dano3_3 の最後の研磨で、上限まで 8 秒 + 同時実行 7.5 秒だった)
            how = " (warm race)";
            lp.solve_race_warm(&lim, crate::simplex::mip_lp::cold_race_threads())
        } else if jcap && basis.is_some() {
            let cap = polish_iter_cap(p.m, p.n);
            let st = lp.solve(&lp::SolveLimits { iteration_limit: cap, ..lim });
            if st == lp::LpStatus::IterationLimit {
                how = " (warm hit the cap, then cold entry)";
                lp.solve_cold_entry(&lim)
            } else {
                st
            }
        } else {
            lp.solve(&lim)
        }
    });
    WARM_POLISH[0].fetch_add(1, Ordering::Relaxed);
    WARM_POLISH[5].fetch_add(lp.total_iterations(), Ordering::Relaxed);
    // 診断 `ENOMOTO_MIP_POLISH_STATS` の `r` (`ENOMOTO_MIP_XOLAZY_LOG` と組): 同時実行の記録 (勝った側と時刻)。1 反復あたりの秒も出す
    if env_str!("ENOMOTO_MIP_POLISH_STATS").is_some_and(|v| v.contains('r')) {
        let it = lp.total_iterations();
        let secs = t.elapsed().as_secs_f64();
        eprintln!(
            "MIP: POLISH_STATS r: {st:?}{how} {it} iters {secs:.3}s ({:.3} ms/iter), race {:?}",
            1e3 * secs / it.max(1) as f64,
            crate::simplex::mip_lp::take_race_diag().map(|d| (d.winner, d.t_win, d.t_ipm, d.t_push, d.t_xo, d.t_end))
        );
    }
    let mut out = WarmPolish::NotSolved;
    if st == lp::LpStatus::Optimal {
        out = WarmPolish::Kept;
        if jcap || solver::rootpar_ph('f') || solver::polish_final('g') {
            POLISH_TOL_LEAK.with(|c| c.set(tol_leak_bound(p, &lp)));
        }
        let mut xn = lp.col_values();
        for j in 0..p.n {
            if p.is_int[j] {
                xn[j] = x[j].round();
            }
        }
        let zn = exact_objective(p, &xn);
        let (vo, vn) = (max_violation(p, x), max_violation(p, &xn));
        let scale = z.abs().max(1.0);
        if env_str!("ENOMOTO_MIP_POLISH_STATS").is_some_and(|v| v.contains('v')) {
            eprintln!(
                "MIP: POLISH_WARM: {st:?}{how} {} iters {:.3}s z {z:.10e} -> {zn:.10e} viol {vo:.3e} -> {vn:.3e} (basis: ints taken out {}, cut rows dropped {}, basic {} of {} rows)",
                lp.total_iterations(),
                t.elapsed().as_secs_f64(),
                info.0,
                info.1,
                info.2,
                p.m
            );
        }
        if (vn <= vo.max(1e-9) && zn <= z + 1e-9 * scale) || vn < vo {
            *x = xn;
            out = WarmPolish::Replaced(zn);
            WARM_POLISH[1].fetch_add(1, Ordering::Relaxed);
        }
    } else {
        WARM_POLISH[2].fetch_add(1, Ordering::Relaxed);
        if env_str!("ENOMOTO_MIP_POLISH_STATS").is_some_and(|v| v.contains('v')) {
            eprintln!("MIP: POLISH_WARM: {st:?}{how} {} iters {:.3}s", lp.total_iterations(), t.elapsed().as_secs_f64());
        }
    }
    WARM_POLISH[4].fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
    out
}

fn polish_solution(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow], p: &MipProblem, x: &mut Vec<f64>, lp: bool) -> f64 {
    for j in 0..p.n {
        if p.is_int[j] {
            x[j] = x[j].round();
        }
    }
    let z = exact_objective(p, x);
    if !lp || p.is_int.iter().all(|&b| b) || env_str!("ENOMOTO_MIP_NO_POLISH").is_some() {
        return z;
    }
    let fixed: Vec<VariableData> = variables
        .iter()
        .enumerate()
        .map(|(j, v)| if p.is_int[j] { VariableData { vtype: VarType::Continuous, lb: x[j], ub: x[j] } } else { v.clone() })
        .collect();
    let r = solve_lp(&fixed, objective, constraints, RootSolver::Simplex, LpOptions::default());
    if let (Status::Optimal, Some(mut xn)) = (r.status, r.x) {
        for j in 0..p.n {
            if p.is_int[j] {
                xn[j] = x[j];
            }
        }
        let zn = exact_objective(p, &xn);
        let (vo, vn) = (max_violation(p, x), max_violation(p, &xn));
        // 違反が増えず目的値も (丸め誤差を除いて) 悪くならなければ採る。違反が真に減るなら、目的値が少し悪くなっても採る:
        // 許容誤差の範囲の違反で目的値が実際の最適値より良く見えていた分 (双対値が大きい行では 1e-9 の違反が 1e-8 程度の
        // 相対差になる。binkar10_1) を正すため
        // 違反の比較には丸め誤差の分 (1e-9) の余裕を持たせる: 元の解の違反がちょうど 0 だと、解き直した解の 1e-15 程度の
        // 丸め誤差だけで捨てていた (bppc4-08 で目的値 59.994 -> 57 の研磨を捨てていた)
        let scale = z.abs().max(1.0);
        // `ENOMOTO_MIP_POLISH_STRICT` (試作): 違反が真に減るなら目的値の悪化の大きさによらず採る。違反は行ごとに許容誤差以下でも、
        // 多くの行で積もると目的値を許容誤差よりずっと大きく動かす (neos-5188808-nattai: 112 行 × 9.5e-7 で 1e-4)
        if (vn <= vo.max(1e-9) && zn <= z + 1e-9 * scale) || (vn < vo && (polish_strict() || zn <= z + 1e-6 * scale)) {
            *x = xn;
            return zn;
        }
    }
    z
}

/// 行・列を種 `seed` の乱数で並べ替えた同値な問題と、列の対応 (新しい列 q は元の列 `cp[q]`) を作る
/// (試作 `ENOMOTO_MIP_PERMRACE` の 2 本目の求解用)
fn permuted_problem(p: &MipProblem, seed: u64) -> (MipProblem, Vec<usize>) {
    let (cp, rp) = random_orders(p, seed);
    (reorder_problem(p, &cp, &rp), cp)
}

/// 種 `seed` の乱数の列の順・行の順
fn random_orders(p: &MipProblem, seed: u64) -> (Vec<usize>, Vec<usize>) {
    let mut st = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(0x1234_5678);
    let mut next = move || {
        st ^= st << 13;
        st ^= st >> 7;
        st ^= st << 17;
        st
    };
    let mut cp: Vec<usize> = (0..p.n).collect();
    for i in (1..p.n).rev() {
        let k = (next() % (i as u64 + 1)) as usize;
        cp.swap(i, k);
    }
    let mut rp: Vec<usize> = (0..p.m).collect();
    for i in (1..p.m).rev() {
        let k = (next() % (i as u64 + 1)) as usize;
        rp.swap(i, k);
    }
    (cp, rp)
}

/// 試作 `ENOMOTO_MIP_ORDINV` の `c` (crate::presolve::ordkey): 行・列を鍵 (並びに依らないハッシュ) の順に並べ直した問題と列の対応
/// (新しい列 q は元の列 `cp[q]`)。鍵が同じなら元の番号の順
fn canonical_problem(p: &MipProblem) -> (MipProblem, Vec<usize>) {
    let rr: Vec<(Vec<(usize, f64)>, f64, f64)> = (0..p.m).map(|i| (p.rows[i].clone(), p.row_lo[i], p.row_up[i])).collect();
    let salt = tunable!("ENOMOTO_T_MIP_ORDINV_SALT", 0u64, u64);
    // 精密化は `ENOMOTO_T_MIP_ORDINV_ROUNDS` 回 (既定 8。色の数が増えなくなれば打ち切る代わりに固定回数)
    let rounds = tunable!("ENOMOTO_T_MIP_ORDINV_ROUNDS", 8usize, usize);
    let (rk, ck) = crate::presolve::ordkey::row_and_column_keys(p.n, &rr, &p.cost, &p.col_lo, &p.col_up, &p.is_int, salt, rounds);
    if env_str!("ENOMOTO_MIP_LOG").is_some() {
        let distinct = |v: &[u64]| v.iter().collect::<std::collections::HashSet<_>>().len();
        eprintln!("MIP: ORDINV c: canonical order by keys ({rounds} rounds): {} distinct column keys of {}, {} distinct row keys of {}", distinct(&ck), p.n, distinct(&rk), p.m);
    }
    let mut cp: Vec<usize> = (0..p.n).collect();
    cp.sort_by_key(|&j| (ck[j], j));
    let mut rp: Vec<usize> = (0..p.m).collect();
    rp.sort_by_key(|&i| (rk[i], i));
    (reorder_problem(p, &cp, &rp), cp)
}

/// 列を `cp` (新しい列 q は元の列 `cp[q]`)、行を `rp` の順に並べ直した同値な問題
fn reorder_problem(p: &MipProblem, cp: &[usize], rp: &[usize]) -> MipProblem {
    let mut inv = vec![0usize; p.n];
    for (q, &j) in cp.iter().enumerate() {
        inv[j] = q;
    }
    let pick = |v: &Vec<f64>| cp.iter().map(|&j| v[j]).collect::<Vec<f64>>();
    let rows: Vec<Vec<(usize, f64)>> = rp
        .iter()
        .map(|&i| {
            let mut r: Vec<(usize, f64)> = p.rows[i].iter().map(|&(j, a)| (inv[j], a)).collect();
            r.sort_unstable_by_key(|e| e.0);
            r
        })
        .collect();
    let mut q = MipProblem::from_rows(
        pick(&p.col_lo),
        pick(&p.col_up),
        pick(&p.cost),
        p.offset,
        p.sense_sign,
        cp.iter().map(|&j| p.is_int[j]).collect(),
        rows,
        rp.iter().map(|&i| p.row_lo[i]).collect(),
        rp.iter().map(|&i| p.row_up[i]).collect(),
    );
    let lit = |l: u32| -> u32 { 2 * inv[(l / 2) as usize] as u32 + (l % 2) };
    q.conflict_edges = p.conflict_edges.iter().map(|&(a, b)| (lit(a), lit(b))).collect();
    q.implied_bounds = p.implied_bounds.iter().map(|&(j, v, k, u, b)| (inv[j as usize] as u32, v, inv[k as usize] as u32, u, b)).collect();
    q
}

/// 試作 `ENOMOTO_MIP_PERMRACE` (既定では使わない。analysis/robust2_20261114.md の「4」): スレッドを半分ずつに分け、元の問題と
/// 行・列を並べ替えた同値な問題 ([`permuted_problem`]) を同時に解く。並べ替えると前処理の後の列の順・同点の崩し方・候補の順が
/// 変わり、流れ (崖のどちら側に落ちるか) が別になる (analysis/robust_20261114.md: 揺れの元は種より並べ替えのほうが大きい)。
/// 見つけた解の目的値は [`solver::Portfolio`] で共有して互いの打ち切りに使い、片方が探索を終えたら (最適・実行不能) もう片方を止める
/// ([`solve_portfolio`] と同じ)。並べ替えた側の解は元の列の並びに戻して返す。値の数字 k があれば並べ替えの種を k にする (既定 1)
fn solve_permrace(p: &MipProblem, params: MipParams) -> solver::MipResult {
    use std::sync::atomic::Ordering;
    let shared = std::sync::Arc::new(solver::Portfolio::new());
    let use_presolve = env_str!("ENOMOTO_MIP_NO_PRESOLVE").is_none();
    let proven: std::sync::Mutex<Option<f64>> = std::sync::Mutex::new(None);
    let seed = env_str!("ENOMOTO_MIP_PERMRACE").and_then(|v| v.trim_matches(|c: char| !c.is_ascii_digit()).parse::<u64>().ok()).unwrap_or(1);
    let (pp, cp) = permuted_problem(p, seed);
    let t0 = params.threads.div_ceil(2);
    let t1 = (params.threads - t0).max(1);
    if params.verbose {
        eprintln!("MIP: PERMRACE: the original problem with {t0} threads and a permuted copy (seed {seed}) with {t1} threads");
    }
    let results: Vec<solver::MipResult> = std::thread::scope(|sc| {
        let hs: Vec<_> = [(p, t0, 0usize), (&pp, t1, 1usize)]
            .into_iter()
            .map(|(q, th, k)| {
                let shared = shared.clone();
                let proven = &proven;
                let params = MipParams { verbose: params.verbose && k == 0, threads: th, ..params };
                std::thread::Builder::new()
                    .stack_size(256 << 20)
                    .spawn_scoped(sc, move || {
                        solver::MIP_THREADS_TL.with(|c| c.set(th));
                        solver::set_thread_portfolio(Some(shared.clone()));
                        let r = solve_problem(q, params, use_presolve);
                        solver::set_thread_portfolio(None);
                        let stopped = shared.stop.load(Ordering::Relaxed);
                        if !stopped && matches!(r.status, MipStatus::Optimal | MipStatus::Infeasible) {
                            let own = r.objective.unwrap_or(f64::INFINITY);
                            *proven.lock().unwrap() = Some(own.min(shared.best()));
                            shared.stop.store(true, Ordering::Relaxed);
                        }
                        r
                    })
                    .expect("failed to spawn a PERMRACE thread")
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    // 並べ替えた側の解を元の列の並びに戻す
    let mut results = results;
    if let Some(x) = results[1].x.take() {
        let mut y = vec![0.0; p.n];
        for (q, &j) in cp.iter().enumerate() {
            y[j] = x[q];
        }
        results[1].x = Some(y);
    }
    if params.verbose {
        for (k, r) in results.iter().enumerate() {
            eprintln!("MIP: PERMRACE: solve {k} returned {:?} objective {:?} bound {:.10e} nodes {}", r.status, r.objective, r.best_bound, r.nodes);
        }
    }
    let proven = *proven.lock().unwrap();
    let tree_bound = results.iter().map(|r| r.best_bound).fold(f64::NEG_INFINITY, f64::max);
    let mut best: Option<&solver::MipResult> = None;
    for r in &results {
        if let Some(z) = r.objective {
            if best.is_none_or(|b| z < b.objective.unwrap()) {
                best = Some(r);
            }
        }
    }
    let nodes = results.iter().map(|r| r.nodes).sum();
    let lp_iterations = results.iter().map(|r| r.lp_iterations).sum();
    match (best, proven) {
        (Some(b), Some(_)) => {
            let z = b.objective.unwrap();
            let sb = shared.best();
            if solver::better_obj(sb, z, z) {
                eprintln!("MIP: WARNING: PERMRACE: the shared best value {sb:.10e} is better than the best returned solution {z:.10e}; not reporting optimality");
                return solver::MipResult { status: MipStatus::NotSolved, x: b.x.clone(), objective: Some(z), best_bound: tree_bound.min(z), nodes, lp_iterations };
            }
            solver::MipResult { status: MipStatus::Optimal, x: b.x.clone(), objective: Some(z), best_bound: z, nodes, lp_iterations }
        }
        (None, Some(_)) => {
            let r = results.iter().find(|r| matches!(r.status, MipStatus::Infeasible | MipStatus::Optimal)).unwrap();
            solver::MipResult { status: r.status, x: None, objective: None, best_bound: r.best_bound, nodes, lp_iterations }
        }
        (Some(b), None) => {
            let z = b.objective.unwrap();
            let status = results.iter().map(|r| r.status).find(|s| *s != MipStatus::Optimal).unwrap_or(b.status);
            solver::MipResult { status, x: b.x.clone(), objective: Some(z), best_bound: tree_bound.min(z), nodes, lp_iterations }
        }
        (None, None) => solver::MipResult { status: results[0].status, x: None, objective: None, best_bound: tree_bound, nodes, lp_iterations },
    }
}

/// ポートフォリオ並列 (`ENOMOTO_MIP_PORTFOLIO`): 設定の違う 2 つの求解を別スレッドで同時に走らせる
/// (0: 既定、1: 以前の形 = DSE 重みを引き継がず最適面の頂点も使わない。得意な問題が分かれる)。
/// 見つけた解の目的値を共有して互いの打ち切りに使い、片方が探索を終えたら (最適・実行不能) もう片方を止める。
/// 片方が「共有の最良値より良い解はない」と示して終われば、もう片方の最良の解が最適。
fn solve_portfolio(p: &MipProblem, params: MipParams) -> solver::MipResult {
    use std::sync::atomic::Ordering;
    let shared = std::sync::Arc::new(solver::Portfolio::new());
    let use_presolve = env_str!("ENOMOTO_MIP_NO_PRESOLVE").is_none();
    // 探索を終えたスレッドが示した「これより良い解はない」値 (そのスレッドの打ち切りに使った値)
    let proven: std::sync::Mutex<Option<f64>> = std::sync::Mutex::new(None);
    // `ENOMOTO_MIP_PORTFOLIO_BISECT`: 3 本目のスレッドで目的値の二分探索を回す ([`bisection_worker`])
    // `ENOMOTO_MIP_BISECT_FRACS`: 二分探索のスレッドごとの最初の目標値の位置 (下界と最良値の間の割合、コンマ区切り。
    // 既定 "0.5,0.25" の 2 本。木の探索のスレッド (割合 1 にあたる) と合わせて 1, 1/2, 1/4 の 3 本で始める)。
    // 示した下界と見つけた解はスレッドの間で共有し、区間が変わったら二分探索のスレッドは新しい区間の同じ割合で解き直す
    let bisect = env_str!("ENOMOTO_MIP_PORTFOLIO_BISECT").is_some();
    let fracs: Vec<f64> = if bisect {
        env_str!("ENOMOTO_MIP_BISECT_FRACS").map(|s| s.split(',').filter_map(|v| v.trim().parse::<f64>().ok()).filter(|v| *v > 0.0 && *v < 1.0).collect()).unwrap_or_else(|| vec![0.5, 0.25])
    } else {
        Vec::new()
    };
    // `ENOMOTO_MIP_PORTFOLIO_TREES`: 木の探索のスレッドの数 (1 なら既定の設定だけ、2 なら以前の形も。
    // 既定は二分探索のスレッドがあれば 1、なければ 2)
    let ntrees = env_str!("ENOMOTO_MIP_PORTFOLIO_TREES").and_then(|v| v.parse::<u8>().ok()).unwrap_or(if bisect { 1 } else { 2 }).clamp(1, 2);
    let bis_result: std::sync::Mutex<Vec<(Option<solver::MipResult>, f64)>> = std::sync::Mutex::new(Vec::new());
    let results: Vec<solver::MipResult> = std::thread::scope(|sc| {
        for &f0 in &fracs {
            let shared = shared.clone();
            let bis_result = &bis_result;
            sc.spawn(move || {
                let r = bisection_worker(p, params, use_presolve, shared, f0);
                bis_result.lock().unwrap().push(r);
            });
        }
        let hs: Vec<_> = (0..ntrees)
            .map(|prof| {
                let shared = shared.clone();
                let proven = &proven;
                let params = MipParams { verbose: params.verbose && prof == 0, ..params };
                sc.spawn(move || {
                    crate::simplex::slope_intercept_dual::set_mip_profile(prof);
                    solver::set_thread_portfolio(Some(shared.clone()));
                    let r = solve_problem(p, params, use_presolve);
                    solver::set_thread_portfolio(None);
                    let stopped = shared.stop.load(Ordering::Relaxed);
                    if !stopped && matches!(r.status, MipStatus::Optimal | MipStatus::Infeasible) {
                        // このスレッドは探索を終えた: 自分の解か共有の最良値より良い解はない
                        let own = r.objective.unwrap_or(f64::INFINITY);
                        *proven.lock().unwrap() = Some(own.min(shared.best()));
                        shared.stop.store(true, Ordering::Relaxed);
                    }
                    r
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut proven = *proven.lock().unwrap();
    // 問題全体の下界は、木の探索のスレッドの下界と、二分探索のスレッドが示した厳密な下界からだけ作る
    // (二分探索の求解が返す下界は打ち切り値で絞った問題のもので、全体の下界ではない)
    let tree_bound = results.iter().map(|r| r.best_bound).fold(f64::NEG_INFINITY, f64::max).max(shared.proven());
    let mut results = results;
    let mut plb = f64::NEG_INFINITY;
    for (r, v) in bis_result.into_inner().unwrap() {
        if let Some(r) = r {
            results.push(r);
        }
        plb = plb.max(v);
    }
    plb = plb.max(shared.proven());
    {
        // 二分探索のスレッドが示した「plb 未満の解はない」: 最良の解がそれに届いていれば最適
        let bz = results.iter().filter_map(|r| r.objective).fold(f64::INFINITY, f64::min);
        if bz.is_finite() && plb >= bz - (params.rel_gap * bz.abs().max(1.0)).max(params.abs_gap) {
            proven = Some(proven.map_or(plb, |v: f64| v.max(plb)));
        }
    }
    // 最良の解
    let mut best: Option<&solver::MipResult> = None;
    for r in &results {
        if let Some(z) = r.objective {
            if best.is_none_or(|b| z < b.objective.unwrap()) {
                best = Some(r);
            }
        }
    }
    let nodes = results.iter().map(|r| r.nodes).sum();
    let lp_iterations = results.iter().map(|r| r.lp_iterations).sum();
    let mut bound = tree_bound;
    match (best, proven) {
        (Some(b), Some(_)) => {
            // 探索を終えたスレッドは、共有の最良値 (= この解以下) より良い解がないことを示した
            let z = b.objective.unwrap();
            // 共有の最良値の解がどのスレッドの結果にもない (届かなかった) なら、この解を最適とは言えない
            let sb = shared.best();
            if solver::better_obj(sb, z, z) && env_str!("ENOMOTO_MIP_NO_FINAL_SHARED_CHECK").is_none() {
                eprintln!("MIP: WARNING: the shared best value {sb:.10e} is better than the best returned solution {z:.10e}; not reporting optimality");
                return solver::MipResult { status: MipStatus::NotSolved, x: b.x.clone(), objective: Some(z), best_bound: sb, nodes, lp_iterations };
            }
            solver::MipResult { status: MipStatus::Optimal, x: b.x.clone(), objective: Some(z), best_bound: z, nodes, lp_iterations }
        }
        (None, Some(_)) => {
            // どちらも解がなく、片方が探索を終えた
            let r = results.iter().find(|r| matches!(r.status, MipStatus::Infeasible | MipStatus::Optimal)).unwrap();
            solver::MipResult { status: r.status, x: None, objective: None, best_bound: r.best_bound, nodes, lp_iterations }
        }
        (Some(b), None) => {
            let z = b.objective.unwrap();
            bound = bound.min(z);
            let status = results.iter().map(|r| r.status).find(|s| *s != MipStatus::Optimal).unwrap_or(b.status);
            solver::MipResult { status, x: b.x.clone(), objective: Some(z), best_bound: bound, nodes, lp_iterations }
        }
        (None, None) => {
            let r = &results[0];
            solver::MipResult { status: r.status, x: None, objective: None, best_bound: bound, nodes, lp_iterations }
        }
    }
}

/// ポートフォリオ並列の目的値の二分探索のスレッド: 共有の最良値 `ub` と木の探索のスレッドの下界 `lb` の間の目標値
/// `t = lb + f (ub - lb)` を打ち切り値にして問題全体を (時間を区切って) 解く。
/// - 解が見つかれば共有の最良値が良くなる (求解の中で知らせる。f は 0.5 に戻す)
/// - 探索を終えて解がなければ「t (と共有の最良値の小さい方) 未満の解はない」と示せたので、下界をそこへ上げる
///   (f は 0.5 に戻す)。最適で終われば、その値未満の解はない (見つけた解が最適)
/// - 時間で終わったら f を暫定解の側へ寄せる
/// 下界が最良値に届いたら全スレッドを止める。戻り値は (見つけた最良の解, 示した下界)。
fn bisection_worker(p: &MipProblem, params: MipParams, use_presolve: bool, shared: std::sync::Arc<solver::Portfolio>, frac0: f64) -> (Option<solver::MipResult>, f64) {
    use std::sync::atomic::Ordering;
    // `ENOMOTO_MIP_BISECT_NO_HEUR`: ヒューリスティクスを走らせず、下界の証明だけに集中する
    // `ENOMOTO_MIP_BISECT_ROOT_HEUR`: 根のヒューリスティクスだけ走らせる (見つけた解は共有の最良値になる)
    crate::simplex::slope_intercept_dual::set_mip_profile(if env_str!("ENOMOTO_MIP_BISECT_NO_HEUR").is_some() {
        2
    } else if env_str!("ENOMOTO_MIP_BISECT_ROOT_HEUR").is_some() {
        3
    } else {
        0
    });
    solver::set_thread_portfolio(Some(shared.clone()));
    let t0 = std::time::Instant::now();
    let tl = params.time_limit;
    let mut frac = frac0;
    let mut proven = f64::NEG_INFINITY;
    let mut best: Option<solver::MipResult> = None;
    let verbose = params.verbose && env_str!("ENOMOTO_MIP_BISECT_VERBOSE").is_some();
    // `ENOMOTO_MIP_BISECT_SNAPSHOT`: 2 回目からの求解は、木の探索のスレッドの探索木の写し ([`solver::TreeSnapshot`]) の
    // 未処理のノードから始める (根の LP・カット・ヒューリスティクスを解き直さない)。写しのノードの部分木を探索し終えるたびに、
    // 木の探索のスレッドはそのノードの子孫の下界を上げる
    let use_snap = env_str!("ENOMOTO_MIP_BISECT_SNAPSHOT").is_some();
    let mut solves = 0u64;
    let mut last_serial = 0u64;
    loop {
        let el = t0.elapsed().as_secs_f64();
        if shared.stop.load(Ordering::Relaxed) || el >= tl - 0.5 {
            break;
        }
        let ub = shared.best();
        proven = proven.max(shared.proven());
        let lb = shared.lb().max(proven);
        if !ub.is_finite() || !lb.is_finite() {
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        }
        let tol = (params.rel_gap * ub.abs().max(1.0)).max(params.abs_gap);
        if lb >= ub - tol {
            // 共有の下界は木の探索のスレッドの待ち行列からの目安で、厳密ではない (処理中のノードを含まない)。
            // 止める合図には使わず、自分で示した下界 (`proven`) だけで判断する
            if proven >= ub - tol {
                shared.stop.store(true, Ordering::Relaxed);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        }
        let t = lb + frac * (ub - lb);
        // 既定 (区間が変わったら解き直す): 時間は区切らず、最良値か厳密な下界が変わったら (どのスレッドが変えても)
        // 求解を打ち切って、新しい区間で同じ割合の目標値から解き直す。`ENOMOTO_MIP_BISECT_SLICED` なら以前の形
        // (時間で区切り、決着がつかなければ割合を最良値の側へ寄せる)
        let sliced = env_str!("ENOMOTO_MIP_BISECT_SLICED").is_some();
        let slice = if sliced { (tunable!("ENOMOTO_T_MIP_BISECT_SLICE", 0.1, f64) * tl).max(2.0).min(tl - el) } else { tl - el };
        let pr = MipParams { time_limit: slice, cutoff: t, verbose: false, ..params };
        // 木の探索のスレッドが根で作ったカットを、この求解の最初に LP に入れる (成り立つ範囲が目標値を含むときだけ)
        if let Some(c) = shared.cuts().filter(|c| t <= c.1 && env_str!("ENOMOTO_MIP_BISECT_SHARE_CUTS").is_some()) {
            RESTART_CUTS.with(|s| *s.borrow_mut() = Some(c.0.clone()));
        }
        if !sliced {
            // 打ち切るのは、厳密な下界が自分の目標値に届いたとき (自分の区間に解がないとほかのスレッドが示した) だけ。
            // 最良値が下がっても打ち切らずに同じ木を続ける (枝刈りの値は共有の最良値まで下がるので、続けても
            // 「min(t, 最良値) 未満の解はない」の証明として正しい。解き直すと、それまでの木を捨てることになる)。
            // `ENOMOTO_MIP_BISECT_ABORT_ON_UB` なら以前の形 (最良値が区間の 1/2 の点を下回ったら打ち切る)
            let trig = if env_str!("ENOMOTO_MIP_BISECT_ABORT_ON_UB").is_some() { lb + 0.5 * (ub - lb) } else { f64::NEG_INFINITY };
            solver::set_thread_watch(Some((trig.min(t), t)));
        }
        let snap = if use_snap && solves > 0 {
            // 新しい写しを求めて待つ (木の探索のスレッドは待ち行列の頭で作る)。来なければ前の写しを使う (古くても
            // 打ち切り値が成り立つ範囲に入っていれば正しい)
            shared.snap_req.store(true, Ordering::Release);
            let w0 = std::time::Instant::now();
            let wait = tunable!("ENOMOTO_T_MIP_BISECT_SNAP_WAIT", 1.0, f64);
            let mut got = None;
            while w0.elapsed().as_secs_f64() < wait && !shared.stop.load(Ordering::Relaxed) && t0.elapsed().as_secs_f64() < tl - 0.5 {
                if let Some(sn) = shared.snapshot().filter(|sn| sn.serial > last_serial) {
                    got = Some(sn);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            got.or_else(|| shared.snapshot()).filter(|sn| t <= sn.valid_below)
        } else {
            None
        };
        solves += 1;
        let from_snap = snap.is_some();
        let snap_info = snap.as_ref().map(|sn| (sn.serial, sn.nodes.len()));
        let r = match snap {
            Some(sn) => {
                last_serial = last_serial.max(sn.serial);
                // 解は写しの問題の空間なので返さない (木の探索のスレッドが受け取る)
                let mut r = solver::solve_snapshot(sn, pr);
                r.x = None;
                r
            }
            None => solve_problem(p, pr, use_presolve),
        };
        solver::set_thread_watch(None);
        let used_cut = t.min(shared.best());
        if verbose {
            eprintln!("MIP: bisection thread: lb {lb:.10e} ub {ub:.10e} target {t:.10e} -> {:?} {:?} bound {:.10e} nodes {} snapshot {:?} ({:.1}s)", r.status, r.objective, r.best_bound, r.nodes, snap_info, t0.elapsed().as_secs_f64());
        }
        // 写しから解いた結果は解 (x) を持たないので、最良の解の候補にしない
        let r = if from_snap { solver::MipResult { objective: r.objective.filter(|&z| z < used_cut), ..r } } else { r };
        let keep = |best: &mut Option<solver::MipResult>, r: solver::MipResult| {
            if let (false, Some(z)) = (from_snap, r.objective) {
                if best.as_ref().is_none_or(|b| z < b.objective.unwrap()) {
                    *best = Some(r);
                }
            }
        };
        match (r.status, r.objective) {
            (MipStatus::Optimal, Some(z)) if z < used_cut => {
                // 打ち切り値未満で最良の解 = 全体の最適 (打ち切り値以上の解はヒューリスティクスが打ち切り値に関係なく
                // 見つけたもので、探索を終えても「t 未満の解はない」ことしか示さない: 下の腕で扱う)
                proven = proven.max(z);
                shared.publish_proven(proven);
                keep(&mut best, r);
                shared.stop.store(true, Ordering::Relaxed);
                break;
            }
            (MipStatus::Infeasible, _) | (MipStatus::Optimal, _) => {
                keep(&mut best, r);
                proven = proven.max(used_cut);
                shared.publish_proven(proven);
                frac = frac0;
            }
            (_, Some(_)) => {
                keep(&mut best, r);
                frac = frac0;
            }
            _ => {
                if sliced {
                    frac = (0.5 * (1.0 + frac)).min(0.9);
                }
            }
        }
    }
    solver::set_thread_portfolio(None);
    (best, proven)
}

/// `p` を (`use_presolve` なら MIP 前処理をかけてから) 分枝限定法で解き、解を `p` の空間に戻す。
/// 戻した解が `p` で実行可能でなければ (前処理の誤りの安全網)、前処理なしで解き直す。
/// サブ MIP (RENS/RINS) もこれで解く。
thread_local! {
    /// 試作 `ENOMOTO_MIP_HO4_CUT0` (analysis/highs_only_20261105/G4.md): 前処理の LP による固定の最初の LP の反復 (このスレッドで最後に解いたもの)
    pub(crate) static HO4_PROBE_ITERS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

thread_local! {
    /// `ENOMOTO_MIP_PROBE_REPRESOLVE` (試作) の外側の求解で、プロービングの後の前処理のかけ直し ([`represolve_after_probe`]) を実際にしたか
    pub(crate) static PRR_APPLIED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 立っていれば [`represolve_after_probe`] はかけ直さない (かけ直した問題の根の LP が破綻した後の解き直し、[`solve_problem`])
    pub(crate) static PRR_OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 外側の求解 (サブ MIP・再スタートでない) の最初の根の LP が二段解法でも内点法でも解けずに諦めたか ([`solver::Solver`] が立てる)
    pub(crate) static ROOT_LP_FAILED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 再スタートの後の求解 (サブ MIP でない) の根の LP が二段解法でも内点法でも解けずに諦めたか ([`solver::Solver`] が立て、再スタートした
    /// 側が [`restart_lpfail_nocuts`] の解き直しの判定に使う)
    pub(crate) static RESTART_ROOT_LP_FAILED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 最後のプロービング ([`probe`]) が固定した列の数 ([`represolve_after_probe`] の条件に使う)
    pub(crate) static PROBE_FIXED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// 試作 `ENOMOTO_MIP_PRR_PRECHECK` ([`represolve_after_probe`]): かけ直しの前処理の深さ・入る前の行の数・捨てる割合。その深さの
    /// [`presolve_mip_depth`] はプロービングの前に行の減り方を見て、捨てられると決まっていればプロービングせずに返す
    pub(crate) static PRR_PRECHECK: std::cell::Cell<Option<(u32, usize, f64)>> = const { std::cell::Cell::new(None) };
    /// 今の前処理がサブ MIP のものか ([`solve_problem_inner`] が `params.submip` で立てる)。既定の `ENOMOTO_MIP_SUBMIP_HIGHS` の `p` では
    /// サブ MIP の前処理にも `submip = false` が渡るので、[`prr_active`] はこちらを見る
    pub(crate) static PRESOLVE_IN_SUBMIP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 今の前処理の求解の乱数の種 (`MipParams::seed`、[`solve_problem_inner`] が立てる)。前処理の中のサブ MIP (VB01) に渡す。
    /// 立っていなければ [`solver::default_seed`]
    pub(crate) static PRESOLVE_SEED: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

/// `ENOMOTO_MIP_PROBE_REPRESOLVE` (試作) がこの前処理で効くか: 環境変数があり、安全網の解き直し ([`PRR_OFF`]) でも FJ だけに渡す問題の
/// 前処理 ([`fj_p3_problem`]) でもなく、サブ MIP の前処理なら `ENOMOTO_MIP_PROBE_REPRESOLVE_SUBMIP` もあるとき。効かないときの前処理は既定とまったく同じ
fn prr_active(submip: bool) -> bool {
    env_def!("ENOMOTO_MIP_PROBE_REPRESOLVE", "1").is_some()
        && !PRR_OFF.with(|f| f.get())
        && !crate::fj_p3_force()
        && (!(submip || PRESOLVE_IN_SUBMIP.with(|f| f.get())) || env_str!("ENOMOTO_MIP_PROBE_REPRESOLVE_SUBMIP").is_some())
}

thread_local! {
    /// 次の [`solve_problem`] に渡す最初の暫定解 (`p` の空間)。開始時に取り出し、前処理後の空間に移して
    /// [`solver::START_SOL`] に置く (再スタートで暫定解を引き継ぐ)。
    pub(crate) static RESTART_SOL: std::cell::RefCell<Option<Vec<f64>>> = const { std::cell::RefCell::new(None) };
    /// 試作 `ENOMOTO_MIP_VBCLQ` の `e`: この求解の前処理で、最初の LP (LP による固定) の前の VB01 と pseudoobj を回したか。
    /// 回していなければ (再スタートの後・LP による固定のない道)、Solver が根の LP (その求解の最初の LP) の前に回す
    pub(crate) static VBCLQ_E_RAN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// `ENOMOTO_MIP_VBCLQ` の `e`: この求解の前処理で、VB01 の解が目的の最小値に届いて最適と証明できた ([`presolve_vbclq_e`])。
    /// 立っていれば前処理の最後の疎化 (`ENOMOTO_MIP_SPARSIFY_LATE`) をかけない (かけ直した問題で解が外れ、最適の解として使われなくなる:
    /// decomp2 の 1 スレッドで木に入って 28.7 秒になっていた)
    pub(crate) static VBCLQ_E_OPT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 再スタート前のカット (`係数 x <= 右辺`)。再スタートが [`solve_problem`] に渡す問題の空間で置き、前処理後の空間に
    /// 移して置き直し、求解の開始時にカットプールに入れる (static TLS の領域が足りないので 1 つで済ませる)。
    pub(crate) static RESTART_CUTS: std::cell::RefCell<Option<Vec<(Vec<(usize, f64)>, f64)>>> = const { std::cell::RefCell::new(None) };
    /// [`super::restart_cuts_fix`] の (1): 再スタート前の求解が [`RESTART_CUTS`] を「根の LP の前にカットの行として戻す」(readd) と
    /// 決めたか。新しい求解は HiGHS 流のカット管理をまだ判定していないので、この印で戻す
    pub(crate) static RESTART_CUTS_READD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// `ENOMOTO_MIP_RESTART_HCM_KEEP` (試作): 再スタート前の求解が HiGHS 流のカット管理を自動で選んだ (`hcm_auto`) か。新しい求解は
    /// 根の LP がカット込みの値から始まるので、強いカットの判定 (カットが根の LP を 3 割以上上げる) がまず立たない
    pub(crate) static RESTART_HCM: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 再スタート前の pseudocost (木の中の再スタートで引き継ぐとき。全体で使う試作 `ENOMOTO_MIP_RESTART_PC` は 2026-11-18 に削除)。再スタートが [`solve_problem`] に渡す問題の空間で置き、前処理後の空間に
    /// 写して置き直し ([`pseudocost::Pseudocost::remap_for_restart`])、再スタート後の求解の開始時に取り出す (static TLS の領域のため Box)
    pub(crate) static RESTART_PC: std::cell::RefCell<Option<Box<pseudocost::Pseudocost>>> = const { std::cell::RefCell::new(None) };
}

/// 目的値 `z` が打ち切り値 `cutoff` (最小化形) 以上か (相対 1e-9 の余裕)。`cutoff` が無限なら偽。
fn at_or_above(z: f64, cutoff: f64) -> bool {
    cutoff.is_finite() && z >= cutoff - 1e-9 * cutoff.abs().max(1.0)
}

/// 一番外側の結果に利用者の打ち切り値 (`ENOMOTO_MIP_CUTOFF`、最小化形) を当てる: 打ち切り値は「これ以上の解は要らない」なので、返す解の値が
/// 打ち切り値以上ならその解は返さない。木は刈り込みの上限を `min(打ち切り値, 暫定解)` にするので、打ち切り値より悪い暫定解 (根の
/// ヒューリスティクスや前処理の解は打ち切り値と比べずに受け入れる) を持ったまま木を刈り尽くして「最適」で終わっても、示したのは
/// 「打ち切り値より良い解はない」だけで、その解が最適だとは言えない (misc07 に打ち切り値 1000 を与えると 3670 を「最適」と返していた。
/// 最適値は 2810。analysis/ti6_20261124)。そこで「最適」は「実行不能 (打ち切り値より良い解はない)」にし、時間・ノードの上限なら状態は
/// そのままで解なしにする。打ち切り値が無限 (既定) なら何もしない。
fn patch_user_cutoff(mut r: solver::MipResult, cutoff: f64, verbose: bool) -> solver::MipResult {
    let Some(z) = r.objective else { return r };
    if !at_or_above(z, cutoff) {
        return r;
    }
    if verbose {
        eprintln!("MIP: the solution {z:.10e} is not better than the cutoff {cutoff:.10e}; not returned");
    }
    r.x = None;
    r.objective = None;
    if r.status == MipStatus::Optimal {
        r.status = MipStatus::Infeasible;
        r.best_bound = f64::INFINITY;
    }
    r
}

pub(crate) fn solve_problem(p: &MipProblem, params: MipParams, use_presolve: bool) -> solver::MipResult {
    let outer = !params.submip && params.restarts == 0;
    // `ENOMOTO_MIP_SUBB_SYM` (試作、既定では使わない): サブ MIP の求解の間は外側の対称性の表を退避し、サブ MIP が自分で検出した表を使う
    // (終わりに外側の表を戻す。[`subb::SymScope`])
    let _subb_sym = (params.submip && params.restarts == 0 && subb::sym_on()).then(subb::SymScope::enter);
    // 診断用のデバッグ解 (`ENOMOTO_MIP_DEBUG_SOL`) は外側の縮約後の空間の解なので、同じスレッドで走るサブ MIP の間は退避する
    // (退避しないと、サブ MIP の行・列に外側の解を当てて「デバッグ解を切った」と誤って出す。終わりに戻す)
    let _dbg_sol = params.submip.then(DebugSolScope::enter);
    // 診断 `ENOMOTO_MIP_GFIX_LOG`: 大域の固定の記録にサブ MIP の中かどうかを付ける
    let _gfix_sub = params.submip.then(domain::GfixSubScope::enter);
    // `ENOMOTO_MIP_SUBMIP_FEASTOL` ([`crate::presolve::submip_feastol_on`]): サブ MIP の前処理の実行不能の判定を MIP の実行可能性の許容誤差より厳しくしない
    let _sub_feastol = (params.submip && crate::presolve::submip_feastol_on()).then(crate::presolve::SubmipFeasTolScope::enter);
    if outer {
        subb::reset();
        heur_subfix::reset();
        // 試作 `ENOMOTO_MIP_OBJSHAVE`: 下界の専任が示した下界の持ち越しを戻す
        solver::OBJSHAVE_CARRY.with(|c| c.set(f64::NEG_INFINITY));
        condrev::decide(p, params.verbose);
    }
    if outer {
        for c in &SST_X_STATS {
            c.store(0, std::sync::atomic::Ordering::Relaxed);
        }
        for c in &heuristics::FP_STATS {
            c.store(0, std::sync::atomic::Ordering::Relaxed);
        }
        ORBITOPES.with(|t| *t.borrow_mut() = None);
        SYM_ORIG.with(|t| *t.borrow_mut() = None);
        SYM_LEX_GENS.with(|t| *t.borrow_mut() = None);
        SYM_SST_CHAIN.with(|t| *t.borrow_mut() = None);
        SYM_OF_GENS.with(|t| *t.borrow_mut() = None);
        symdyn::reset();
        SYM_CUT_GENS.with(|t| *t.borrow_mut() = None);
        EQUIV2_USED.with(|f| f.set(false));
        CLQPRB_STICKY.with(|f| f.set(false));
        CLQPRB_REVERTED.with(|f| f.set(false));
        PRBWORK_DRY.with(|f| f.set(false));
        PROBREP_LAST.with(|c| *c.borrow_mut() = None);
        FEAS_EARLY_DRY.with(|f| f.set(false));
        cutmix::reset();
    }
    let prr = outer && env_def!("ENOMOTO_MIP_PROBE_REPRESOLVE", "1").is_some();
    if prr {
        PRR_APPLIED.with(|f| f.set(false));
        PRR_OFF.with(|f| f.set(false));
        ROOT_LP_FAILED.with(|f| f.set(false));
    }
    // 試作 `ENOMOTO_MIP_HNFPRE_LAT` (既定では使わない): 整数列を格子の簡約した基底で置き換えた問題を解いて戻す ([`hnflat`])
    if outer && hnflat::on() && !HNF_LAT_ACTIVE.with(|f| f.get()) {
        if let Some(lr) = hnflat::reform(p, params.verbose) {
            HNF_LAT_ACTIVE.with(|f| f.set(true));
            let t1 = std::time::Instant::now();
            let mut r = solve_problem(&lr.q, MipParams { time_limit: (params.time_limit - lr.lll_secs).max(0.0), ..params }, use_presolve);
            HNF_LAT_ACTIVE.with(|f| f.set(false));
            if let Some(y) = r.x.take() {
                let x = lr.map_back(p.n, &y);
                if p.is_feasible(&x, 1e-6) {
                    r.objective = Some(p.objective(&x));
                    r.x = Some(x);
                } else {
                    // 安全網: 戻した解が元の問題で実行可能でなければ、書き直さずに残りの時間で解き直す
                    eprintln!("HNFPRE_LAT: the mapped solution is not feasible in the original problem; solving the original");
                    let left = (params.time_limit - lr.lll_secs - t1.elapsed().as_secs_f64()).max(0.0);
                    HNF_LAT_ACTIVE.with(|f| f.set(true));
                    let r2 = solve_problem(p, MipParams { time_limit: left, ..params }, use_presolve);
                    HNF_LAT_ACTIVE.with(|f| f.set(false));
                    return r2;
                }
            }
            return r;
        }
    }
    // 試作 `ENOMOTO_MIP_FOLD_COUNT` (診断): サブ MIP・再スタートの後の問題で畳める構造を数える
    if params.submip || params.restarts > 0 {
        fold::count_hook(p, &p.col_lo, &p.col_up, &if params.submip { "submip".to_string() } else { format!("restart {}", params.restarts) });
    }
    let t0 = std::time::Instant::now();
    let mut r = solve_problem_inner(p, params, use_presolve);
    // `ENOMOTO_MIP_PROBE_REPRESOLVE` (試作) の安全網 (既定では通らない): かけ直した問題の根の LP が二段解法でも内点法でも解けずに
    // 諦めた (`NotSolved`) なら、かけ直しをせずに残りの時間で解き直す (暫定解は引き継ぐ)。momentum1 はかけ直した問題 (行 24303 ->
    // 14953) の根の LP の基底が特異になって 20 秒ほどで破綻し、34 秒で `not_solved` (下界なし) を返していた
    // (analysis/prr_harm_20261106.md)。かけ直す前の問題の根の LP は破綻しない (60 秒では解き終わらない)
    if prr && r.status == MipStatus::NotSolved && PRR_APPLIED.with(|f| f.get()) && ROOT_LP_FAILED.with(|f| f.get()) {
        let left = params.time_limit - t0.elapsed().as_secs_f64();
        if left > 1.0 && !crate::cancel::is_cancelled() {
            if params.verbose {
                eprintln!("MIP: PROBE_REPRESOLVE: the root LP of the re-presolved problem failed; solving again without re-presolving ({left:.2}s left)");
            }
            solver::ROOT_BASIS.with(|t| *t.borrow_mut() = None);
            solver::ROOT_DSE.with(|t| *t.borrow_mut() = None);
            solver::ROOT_BASIS_ROWS.with(|t| *t.borrow_mut() = None);
            solver::ROOT_BASIS_SAVED.with(|t| *t.borrow_mut() = None);
            ORBITOPES.with(|t| *t.borrow_mut() = None);
            SYM_ORIG.with(|t| *t.borrow_mut() = None);
            SYM_LEX_GENS.with(|t| *t.borrow_mut() = None);
            SYM_SST_CHAIN.with(|t| *t.borrow_mut() = None);
            SYM_OF_GENS.with(|t| *t.borrow_mut() = None);
            symdyn::reset();
            SYM_CUT_GENS.with(|t| *t.borrow_mut() = None);
            RESTART_SOL.with(|s| *s.borrow_mut() = r.x.clone().filter(|x| x.len() == p.n));
            PRR_OFF.with(|f| f.set(true));
            let r2 = solve_problem_inner(p, MipParams { time_limit: left, ..params }, use_presolve);
            PRR_OFF.with(|f| f.set(false));
            // 解き直しが暫定解を失ったときは元の暫定解を残す
            if r2.x.is_some() || r.x.is_none() {
                r = solver::MipResult { nodes: r.nodes + r2.nodes, lp_iterations: r.lp_iterations + r2.lp_iterations, ..r2 };
            }
        }
    }
    if outer {
        if params.verbose {
            subb::report();
            if heur_subfix::any() {
                eprintln!("MIP: SUBFIX summary: {}", heur_subfix::summary());
            }
        }
        // 診断 `ENOMOTO_MIP_FP_STATS`: Feasibility Pump の回数 ([`heuristics::FP_STATS`])
        if env_str!("ENOMOTO_MIP_FP_STATS").is_some() {
            let c: Vec<u64> = heuristics::FP_STATS.iter().map(|c| c.load(std::sync::atomic::Ordering::Relaxed)).collect();
            eprintln!(
                "MIP: FP_STATS calls {} found {} passes {} conflicts {} caps {} fixed_not_found {} | ends integral_lp {} rounding {} cycle {} budget {} lp_stop {} passes100 {} | conflict_pos_permille_avg {}",
                c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], c[8], c[9], c[10], c[11], if c[3] > 0 { c[12] / c[3] } else { 0 }
            );
        }
        tpre::clear_pending();
        // 試作 `ENOMOTO_MIP_SYM_SST=x` の写しの回数 (計測で数えるため、ログの有無にかかわらず出す)
        if sym_sst_x() {
            let c: Vec<u64> = SST_X_STATS.iter().map(|c| c.load(std::sync::atomic::Ordering::Relaxed)).collect();
            eprintln!("MIP: SST=x: mapped {}, failed (rows still violated) {}, failed (check) {}, dropped after a restart {}, not better than the cutoff {}", c[0], c[1], c[2], c[3], c[4]);
        }
        ORBITOPES.with(|t| *t.borrow_mut() = None);
        SYM_ORIG.with(|t| *t.borrow_mut() = None);
        SYM_LEX_GENS.with(|t| *t.borrow_mut() = None);
        SYM_SST_CHAIN.with(|t| *t.borrow_mut() = None);
        SYM_OF_GENS.with(|t| *t.borrow_mut() = None);
        symdyn::reset();
        SYM_CUT_GENS.with(|t| *t.borrow_mut() = None);
        solver::ROOT_BASIS.with(|t| *t.borrow_mut() = None);
        solver::ROOT_DSE.with(|t| *t.borrow_mut() = None);
        solver::RESTART_BASIS.with(|t| *t.borrow_mut() = None);
        solver::ROOT_BASIS_ROWS.with(|t| *t.borrow_mut() = None);
        solver::ROOT_BASIS_SAVED.with(|t| *t.borrow_mut() = None);
        solver::RESTART_X.with(|t| *t.borrow_mut() = None);
    }
    r
}

/// `ENOMOTO_MIP_RESTART_LPFAIL_NOCUTS` (analysis/ab72_20261121。**既定で使う: `=0`・`off` で使わない (前の形)**): 再スタートでカットを
/// 問題の行として持ち込んだ求解の根の LP が、二段解法でも内点法 + クロスオーバーでも解けずに諦めた (`NotSolved`) なら、持ち込んだカットの
/// 行を除いた問題 (元の行と再スタートの大域的な境界だけ) で残りの時間で解き直す。カットは大域的に成り立つ不等式なので、除いても
/// 解の集合は変わらない。ab72-40-100 (4 スレッド) は根のカットのうち係数の幅が 1e4 ほどの密な行 (4000 列超) 4 本が持ち込まれ、目的の係数
/// (4e9 まで) と合わさって再スタートの後の根の LP が基底なしでは解けず (HiGHS でも同じ LP で「双対値が大きすぎる」誤り)、`not_solved` で止まっていた
pub(crate) fn restart_lpfail_nocuts() -> bool {
    env_def!("ENOMOTO_MIP_RESTART_LPFAIL_NOCUTS", "1").is_some()
}

/// `ENOMOTO_MIP_RESTART_DROP_FIXED` (analysis/objtarget_20261119/ の 10 節。**2026-11-20 に既定にした: `=0`・`off` で使わない (前の形)**): 再スタートで渡す問題から、大域的に
/// 固定された列を落として列の番号を詰める (固定された列の値は行の両辺と目的の定数へ移す)。再スタートの後の前処理は列の番号を保つので、
/// 落とさないと固定された列も前処理・プロービングの定義域・FJ_P3 の前処理などで列の数だけ回る (nw04: 87482 列のうち 81107 列が
/// 再スタートで固定され、自由な列は 1842 本)。解は [`DropFixed::expand`] で元の番号へ戻す。
/// 列の番号つきで再スタートの後の求解へ渡すもののうち、暫定解 ([`RESTART_SOL`]) とプールのカット ([`RESTART_CUTS`])、
/// 根の LP の基底・LP の解も写す。写さないもの (木の中の再スタートの pseudocost・含意・対称性の表) が渡されるときは落とさない
pub(crate) struct DropFixed {
    pub(crate) q: MipProblem,
    /// 新しい列 -> 元の列
    keep: Vec<usize>,
    /// 元の列の数だけの値 (固定された列は固定の値、ほかは 0)
    base: Vec<f64>,
    /// 子の求解の間だけ詰めた問題に置き換えた [`SYM_ORIG`] の元の値 (戻すため)
    saved_orig: Option<Option<std::rc::Rc<MipProblem>>>,
    /// [`RESTART_NINT_BASE`] の元の値 (戻すため)
    prev_base: usize,
}

impl DropFixed {
    pub(crate) fn expand(&mut self, mut r: solver::MipResult) -> solver::MipResult {
        if let Some(o) = self.saved_orig.take() {
            SYM_ORIG.with(|t| *t.borrow_mut() = o);
        }
        RESTART_NINT_BASE.with(|c| c.set(self.prev_base));
        if let Some(y) = r.x.take() {
            if y.len() == self.q.n {
                let mut x = self.base.clone();
                for (k, &j) in self.keep.iter().enumerate() {
                    x[j] = y[k];
                }
                r.x = Some(x);
            } else {
                r.x = Some(y);
            }
        }
        r
    }
}

thread_local! {
    /// [`restart_drop_fixed`] で列を詰める前の整数列の数 (再スタートの判定の分母、[`restart_nint_base`])
    static RESTART_NINT_BASE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// 再スタートの判定の分母に使う整数列の数の下限 ([`restart_drop_fixed`] で詰めた子の求解の間だけ 0 でない)。詰めると子の問題の整数列の数
/// (`p.nint`) は自由な列だけになり、「整数列の 2.5%」の分母が小さくなって続けて再スタートしやすくなる (nw04 の pl で 1 → 4 回)。
/// 詰めない形では前処理が列の番号を保つので分母は最初の問題の整数列の数のままなので、それに合わせる
pub(crate) fn restart_nint_base() -> usize {
    RESTART_NINT_BASE.with(|c| c.get())
}

pub(crate) fn restart_drop_fixed_on() -> bool {
    env_def!("ENOMOTO_MIP_RESTART_DROP_FIXED", "1").is_some()
}

/// [`DropFixed`] を作る (落とせないとき・落とす列がないときは `None`)。[`RESTART_SOL`]・[`RESTART_CUTS`] はここで詰めた番号に写す
pub(crate) fn restart_drop_fixed(p: &MipProblem, verbose: bool) -> Option<DropFixed> {
    if !restart_drop_fixed_on() {
        return None;
    }
    let empty = |v: Option<usize>| v.is_none_or(|k| k == 0);
    let why = if RESTART_PC.with(|s| s.borrow().is_some()) {
        Some("pseudocosts carried")
    } else if !p.conflict_edges.is_empty() || !p.implied_bounds.is_empty() {
        Some("implications carried")
    } else if !(empty(ORBITOPES.with(|t| t.borrow().as_ref().map(|o| o.len())))
        && empty(SYM_OF_GENS.with(|t| t.borrow().as_ref().map(|g| g.len())))
        && empty(SYM_LEX_GENS.with(|t| t.borrow().as_ref().map(|g| g.len())))
        && empty(SYM_SST_CHAIN.with(|t| t.borrow().as_ref().map(|g| g.len())))
        && empty(SYM_CUT_GENS.with(|t| t.borrow().as_ref().map(|g| g.len())))
        && SYM_ORIG.with(|t| t.borrow().as_ref().is_none_or(|o| o.n == p.n))
        && symdyn::refl_gens().is_empty())
    {
        Some("symmetry tables")
    } else {
        None
    };
    if let Some(w) = why {
        if verbose {
            eprintln!("MIP: RESTART_DROP_FIXED: not used ({w})");
        }
        return None;
    }
    let keep: Vec<usize> = (0..p.n).filter(|&j| p.col_lo[j] < p.col_up[j]).collect();
    if keep.len() == p.n {
        return None;
    }
    let mut map = vec![usize::MAX; p.n];
    for (k, &j) in keep.iter().enumerate() {
        map[j] = k;
    }
    let mut base = vec![0.0; p.n];
    let mut offset = p.offset;
    for j in 0..p.n {
        if map[j] == usize::MAX {
            base[j] = p.col_lo[j];
            offset += p.cost[j] * p.col_lo[j];
        }
    }
    // 問題 `a` (列の並びは `p` と同じ) の列を詰める (固定された列は `base` の値で行の両辺・目的の定数へ移す)
    let squeeze = |a: &MipProblem, offset: f64| -> MipProblem {
        let (mut rows, mut row_lo, mut row_up) = (Vec::with_capacity(a.m), Vec::with_capacity(a.m), Vec::with_capacity(a.m));
        for i in 0..a.m {
            let mut c = 0.0;
            let r: Vec<(usize, f64)> = a.rows[i]
                .iter()
                .filter_map(|&(j, v)| {
                    if map[j] == usize::MAX {
                        c += v * base[j];
                        None
                    } else {
                        Some((map[j], v))
                    }
                })
                .collect();
            rows.push(r);
            row_lo.push(a.row_lo[i] - c);
            row_up.push(a.row_up[i] - c);
        }
        let sel = |v: &[f64]| keep.iter().map(|&j| v[j]).collect::<Vec<f64>>();
        MipProblem::from_rows(sel(&a.col_lo), sel(&a.col_up), sel(&a.cost), offset, a.sense_sign, keep.iter().map(|&j| a.is_int[j]).collect(), rows, row_lo, row_up)
    };
    let nnz0 = p.nnz;
    let q = squeeze(p, offset);
    // 子の求解のヒューリスティクス・FJ のスレッドが使う最初の求解の問題 ([`SYM_ORIG`]、対称性の生成元がなくても置かれる) も詰める。
    // 落とす列は再スタートの問題で固定された値に固定する (再スタートの問題の解はこの値なので、詰めた問題は子の問題の緩和のまま)
    let saved_orig = SYM_ORIG.with(|t| {
        let o = t.borrow().clone();
        let o2 = o.as_ref().map(|o| {
            let off = o.offset + (0..o.n).filter(|&j| map[j] == usize::MAX).map(|j| o.cost[j] * base[j]).sum::<f64>();
            std::rc::Rc::new(squeeze(o, off))
        });
        *t.borrow_mut() = o2;
        o
    });
    RESTART_SOL.with(|s| {
        if let Some(x) = s.borrow_mut().as_mut() {
            if x.len() == p.n {
                *x = keep.iter().map(|&j| x[j]).collect();
            }
        }
    });
    // 根の LP の基底 (`ENOMOTO_MIP_RESTART_BASIS`): 列の状態は残す列だけ、行は固定された列を除いた係数にする (行は係数で照らし合わせる)
    solver::RESTART_BASIS.with(|s| {
        if let Some((cols, rws)) = s.borrow_mut().as_mut() {
            if cols.len() == p.n {
                *cols = keep.iter().map(|&j| cols[j]).collect();
            }
            for (r, _) in rws.iter_mut() {
                *r = std::mem::take(r).into_iter().filter_map(|(j, a)| (j < p.n && map[j] != usize::MAX).then(|| (map[j], a))).collect();
            }
        }
    });
    solver::RESTART_X.with(|s| {
        if let Some(x) = s.borrow_mut().as_mut() {
            if x.len() == p.n {
                *x = keep.iter().map(|&j| x[j]).collect();
            }
        }
    });
    let mut ncuts = (0usize, 0usize);
    RESTART_CUTS.with(|s| {
        if let Some(cs) = s.borrow_mut().as_mut() {
            ncuts.0 = cs.len();
            *cs = std::mem::take(cs)
                .into_iter()
                .filter_map(|(c, r)| {
                    let mut rr = r;
                    let cc: Vec<(usize, f64)> = c.into_iter().filter_map(|(j, a)| if j < p.n && map[j] != usize::MAX { Some((map[j], a)) } else { rr -= a * base.get(j).copied().unwrap_or(0.0); None }).collect();
                    (!cc.is_empty()).then_some((cc, rr))
                })
                .collect();
            ncuts.1 = cs.len();
        }
    });
    if verbose {
        eprintln!("MIP: RESTART_DROP_FIXED: columns {} -> {}, nonzeros {} -> {}, pool cuts {} -> {}", p.n, q.n, nnz0, q.nnz, ncuts.0, ncuts.1);
    }
    let prev_base = RESTART_NINT_BASE.with(|c| c.replace(c.get().max(p.nint)));
    Some(DropFixed { q, keep, base, saved_orig: Some(saved_orig), prev_base })
}

/// 行の並び (列番号と係数) の指紋 ([`solver::ROOT_BASIS_ROWS`] で、基底を作った LP と根の LP の行が同じかを見る)。
pub(crate) fn rows_fingerprint(rows: &[Vec<(usize, f64)>]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    rows.len().hash(&mut h);
    for r in rows {
        r.len().hash(&mut h);
        for &(j, v) in r {
            j.hash(&mut h);
            v.to_bits().hash(&mut h);
        }
    }
    h.finish()
}

fn solve_problem_inner(p: &MipProblem, params: MipParams, use_presolve: bool) -> solver::MipResult {
    wc_mark("before the solve", params.submip);
    let start = RESTART_SOL.with(|s| s.borrow_mut().take());
    if !params.submip {
        // 試作 `ENOMOTO_MIP_VBCLQ` の `e`: この求解の前処理で回したかの印を消す (前の求解の残りを持ち越さない)
        VBCLQ_E_RAN.with(|f| f.set(false));
        VBCLQ_E_OPT.with(|f| f.set(false));
    }
    let t_pre = std::time::Instant::now();
    if !params.submip && (params.restarts == 0 || pretime::restart_on()) {
        pretime::start();
    }
    // 試作 `ENOMOTO_MIP_TPRE` ([`tpre`]): 並列の「仮の暫定値つき前処理」を 1 本のスレッドで始める (前処理の後で待って取り込む)
    // `t` (pt): 空いたスレッドで FJ・Local-MIP を前処理の間回し続ける (手間の門で再現する)
    let pt_job = (tpre::mode().is_some_and(|m| m.contains('t') || m.contains('T')) && !params.submip && params.restarts == 0 && params.threads >= 2 && start.is_none())
        .then(|| tpre::pt_spawn(p, params.threads, 0x7E1E_5EED ^ params.seed));
    // `ENOMOTO_T_MIP_TPRE_MIN_NNZ` (既定 0): 非零がこれより少ない問題では TPRE (p・pl) を使わない (小さい問題は前処理がすぐ終わり、主が待つ分が目立つ。
    // PPL の計測で 10teams・misc07 など 1〜5 秒の問題が数倍遅かった)
    let tpre_big = p.rows.iter().map(|r| r.len()).sum::<usize>() as f64 >= tunable!("ENOMOTO_T_MIP_TPRE_MIN_NNZ", 0.0, f64);
    let tpre_job = (tpre_big && tpre::mode().is_some_and(|m| !m.contains('t') && !m.contains('T')) && !params.submip && params.restarts == 0 && params.threads >= 2 && start.is_none()).then(|| {
        let q = p.clone();
        let safety = if params.time_limit.is_finite() { 0.25 * params.time_limit } else { 60.0 };
        let seed = 0x7E1E_5EED ^ params.seed;
        let stop: tpre::Stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let slot: tpre::SolSlot = std::sync::Arc::new(std::sync::Mutex::new(None));
        let (st2, sl2) = (stop.clone(), slot.clone());
        // `g`: 手間の門を付けて、主の手間で読む中身を決める (種ごとに再現)
        let gate = tpre::mode().is_some_and(|m| m.contains('g')).then(|| crate::ticks::Gate::new(0.0));
        if let Some(g) = gate.as_ref() {
            tpre::set_gated(g.clone(), slot.clone());
        }
        (std::thread::spawn(move || {
            let _gg = gate.map(crate::ticks::install_gate);
            tpre::run(&q, safety, seed, st2, sl2)
        }), stop, slot)
    });
    let prev_par = PREPAR_THREADS.with(|t| t.replace(if !params.submip && params.restarts == 0 { params.threads.max(1) } else { 1 }));
    struct RestorePar(usize);
    impl Drop for RestorePar {
        fn drop(&mut self) {
            PREPAR_THREADS.with(|t| t.set(self.0));
        }
    }
    let _restore_par = RestorePar(prev_par);
    // 試作 `ENOMOTO_MIP_PRECHAIN` ([`prechain`]): 最初の求解の前処理の途中で FJ・Local-MIP のスレッドを立てる用意をし、戻るときに止める
    struct Disarm(bool);
    impl Drop for Disarm {
        fn drop(&mut self) {
            if self.0 {
                prechain::disarm();
            }
        }
    }
    let _prechain = Disarm(!params.submip && params.restarts == 0 && prechain::on());
    if _prechain.0 {
        prechain::arm(params.threads, params.time_limit, params.verbose, params.seed);
    }
    // 前処理の重い手順の期限 ([`presolve_stop`]): この求解の時間の上限。戻るときに前の値 (外側の求解のもの) に戻す
    struct RestoreDeadline(Option<std::time::Instant>);
    impl Drop for RestoreDeadline {
        fn drop(&mut self) {
            PRESOLVE_DEADLINE.with(|d| d.set(self.0));
        }
    }
    let _restore = RestoreDeadline(PRESOLVE_DEADLINE.with(|d| d.replace(params.time_limit.is_finite().then(|| t_pre + std::time::Duration::from_secs_f64(params.time_limit.max(0.0))))));
    if !params.submip && params.restarts == 0 {
        enum_adapt_reset(t_pre);
    }
    // 時間上限は前処理・対称性の検出の時間も含める (分枝限定法の時計は `solver::solve` の開始から測るので、渡す前に引く)
    let remaining = |params: MipParams| MipParams { time_limit: (params.time_limit - t_pre.elapsed().as_secs_f64()).max(0.0), ..params };
    // 前処理の LP による固定 ([`lp_probe`]) は最初の求解だけ (サブ MIP・再スタートの後は使わない)
    // 試作 `ENOMOTO_MIP_VBCLQ` の `l` (analysis/scipgate_20261120): 前処理の LP による固定をしない (根の LP の前の vbounds の解と
    // pseudoobj の固定の後で、B&B の根の LP を小さく解くため。hypothyroid-k1 では LP probing が固定の前に根の LP を丸ごと解く)
    let lp_probe_secs = (env_str!("ENOMOTO_MIP_NO_LP_PROBE").is_none() && !solver::vbclq().is_some_and(|v| v.contains('l')) && !params.submip && params.restarts == 0)
        .then(|| if params.time_limit.is_finite() { tunable!("ENOMOTO_T_MIP_LP_PROBE_TIME_FRAC", 0.05, f64) * params.time_limit } else { 10.0 })
        // `ENOMOTO_MIP_PRESOLVE_WORK`: 打ち切りは LP の手間で決め ([`lp_probe`])、時間は安全網 (`ENOMOTO_T_MIP_PRESOLVE_WORK_LP_SECS` 秒、
        // 既定 30 秒。時間制限の半分まで) だけにする (時間制限で前処理の結果が変わらないように)
        .map(|s| if presolve_work() { tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_LP_SECS", 30.0, f64).min(if params.time_limit.is_finite() { 0.5 * params.time_limit } else { f64::INFINITY }) } else { s });
    let prev_rp = RESTART_PRESOLVE.with(|f| f.replace(params.restarts > 0));
    // `ENOMOTO_MIP_SUBMIP_HIGHS` の `p`: サブ MIP でも親と同じ前処理 (HiGHS の `solveSubMip` は親と同じ前処理の設定で解く)
    let pre_submip = params.submip && !submip_highs('p');
    FEAS_FJ_SOL.with(|s| *s.borrow_mut() = None);
    FEAS_FJ_EARLY.with(|c| {
        c.set((feas_fj_early_on() && lp_probe_secs.is_some() && !UNB_CHECK.with(|f| f.get())).then(|| {
            let cap = if params.time_limit.is_finite() { tunable!("ENOMOTO_T_MIP_FEAS_FJ_EARLY_FRAC", 0.1, f64) * params.time_limit } else { 10.0 };
            (params.threads.max(1), cap)
        }))
    });
    let prev_in_sub = PRESOLVE_IN_SUBMIP.with(|f| f.replace(params.submip));
    let prev_seed = PRESOLVE_SEED.with(|f| f.replace(Some(params.seed)));
    // 診断 `ENOMOTO_MIP_SYMREFL_PRE_DIAG` (analysis/symdyn_20261110.md): 前処理の前の問題で反転を含む対称性を数える (求解は変えない)
    if env_str!("ENOMOTO_MIP_SYMREFL_PRE_DIAG").is_some() && !params.submip && params.restarts == 0 {
        let t = std::time::Instant::now();
        let g = symdyn::detect_reflections(p, tunable!("ENOMOTO_T_MIP_SYMREFL_PRE_TIME", 5.0, f64), false);
        let cols: std::collections::BTreeSet<u32> = g.iter().flat_map(|s| s.moves.iter().filter(|m| m.2).map(|m| m.0)).collect();
        eprintln!("MIP: SYMREFL_PRE before presolve: {} reflection generators, {} reflected columns of {} ({:.2}s)", g.len(), cols.len(), p.n, t.elapsed().as_secs_f64());
        if env_str!("ENOMOTO_MIP_SYMREFL_PRE_DIAG").is_some_and(|v| v.contains('v')) {
            for (k, s) in g.iter().enumerate().take(4) {
                for &(v, b, r) in s.moves.iter().take(24) {
                    let (v, b) = (v as usize, b as usize);
                    eprintln!("MIP: SYMREFL_PRE gen {k}: col {v} -> {b}{} [{}, {}] int {} cost {} nnz {}", if r { " (reflected)" } else { "" }, p.col_lo[v], p.col_up[v], p.is_int[v], p.cost[v], p.cols[v].len());
                }
            }
        }
    }
    // 診断 `ENOMOTO_MIP_SYMPRE0_DIAG` (analysis/sympre0_20261111.md): 前処理の前 (DRCR の後) の問題で置換の対称性を数える (求解は変えない)
    if env_str!("ENOMOTO_MIP_SYMPRE0_DIAG").is_some() && !params.submip && params.restarts == 0 {
        sympre0::diag(p, "before");
    }
    let presolved = if use_presolve { presolve_mip(p, params.verbose, pre_submip, lp_probe_secs) } else { None };
    wc_mark("presolve", params.submip);
    // `ENOMOTO_MIP_CLQPRB` の `h` (試作、analysis/prbimp_20261113.md §4): `g` で含意を使う形に切り替えた外側の求解では、既定の形の前処理を
    // もう一度回した問題で locks の sub-MIP を解き、元の空間に戻した解を最初の暫定解の候補にする ([`clqprb_shadow_locks`])。
    // 前処理後の空間に写して実行可能なときだけ使う (下の `start` の扱い)
    let start = if start.is_none() && use_presolve && clqprb_on() && clqprb_has('h') && !params.submip && params.restarts == 0 && CLQPRB_STICKY.with(|f| f.get()) && matches!(presolved, Some(Presolved::Reduced { .. })) {
        clqprb_shadow_locks(p, params, pre_submip, lp_probe_secs)
    } else {
        start
    };
    pretime::mark("presolve_mip");
    // 診断 `ENOMOTO_MIP_DEBUG_PRESOLVE_COLS=ファイル` (CLQPRB、analysis/clqprb_20261112.md): 最初の前処理の後の列ごとに
    // (番号, 下限, 上限, 残った行の非零の数) を書く。前処理は列の番号を保つので、HiGHS の前処理で消えた列と突き合わせられる
    if let (Some(_), Some(Presolved::Reduced { prob, .. })) = (env_str!("ENOMOTO_MIP_DEBUG_INTCENSUS"), &presolved) {
        if !params.submip && params.restarts == 0 {
            int_census(prob, &prob.col_lo, &prob.col_up, None, "b_probing");
        }
    }
    if let (Some(_), Some(Presolved::Reduced { prob, .. })) = (env_str!("ENOMOTO_MIP_DEBUG_CLQACT"), &presolved) {
        if !params.submip && params.restarts == 0 {
            clqact_census(prob, "b_probing");
        }
    }
    if let (Some(_), Some(Presolved::Reduced { prob, .. })) = (env_str!("ENOMOTO_MIP_DEBUG_CLQTAB"), &presolved) {
        if !params.submip && params.restarts == 0 {
            clqtab_census(prob, "b_probing");
        }
    }
    if let (Some(path), Some(Presolved::Reduced { prob, .. })) = (env_str!("ENOMOTO_MIP_DEBUG_PRESOLVE_COLS"), &presolved) {
        if !params.submip && params.restarts == 0 && prob.n == p.n {
            use std::io::Write;
            if let Ok(mut f) = std::fs::File::create(path) {
                for j in 0..prob.n {
                    let _ = writeln!(f, "{j}\t{}\t{}\t{}", prob.col_lo[j], prob.col_up[j], prob.cols[j].len());
                }
            }
        }
    }
    PRESOLVE_IN_SUBMIP.with(|f| f.set(prev_in_sub));
    PRESOLVE_SEED.with(|f| f.set(prev_seed));
    FEAS_FJ_EARLY.with(|c| c.set(None));
    RESTART_PRESOLVE.with(|f| f.set(prev_rp));
    // `ENOMOTO_MIP_SPARSIFY_LATE` (既定 `s`、`=0` で切る。docs/improvement_history.md の 2026-11-18 の節): HiGHS と同じく前処理 (プロービング・
    // 同値の代入の後) の最後に 1 回だけ疎化し ([`sparsify_rows_highs`]、`=old` なら [`sparsify_rows`])、書き換えたら前処理をもう一度かける
    // (その中では疎化しない)。`s` を含むときは下の条件 (縮んだ行の割合 f ≥ 0.6) を満たすときだけ疎化した問題を使う。`=1` は条件なし (試作の比べ用)
    let spl_mode = env_def!("ENOMOTO_MIP_SPARSIFY_LATE", "s");
    let presolved = match presolved {
        Some(Presolved::Reduced { prob, postsolve, scaling }) if spl_mode.is_some() && !params.submip && params.restarts == 0 && !VBCLQ_E_OPT.with(|f| f.get()) => {
            let MipProblem { col_lo, col_up, cost, offset, sense_sign, is_int, mut rows, mut row_lo, mut row_up, .. } = prob.clone();
            let k = if spl_mode == Some("old") {
                sparsify_rows(&mut rows, &mut row_lo, &mut row_up, &is_int, prob.n)
            } else {
                sparsify_rows_highs(&mut rows, &mut row_lo, &mut row_up, &is_int, &col_lo, &col_up, prob.n)
            };
            // gate2 候補 6 (analysis/gate2_20261117/sparsify/README.md): 書き換えた行のうち 0-1 列だけの行・1 列以下に縮んだ (行が消えて境界になる) 行、
            // 非零の増減を数える。`s` (既定) なら、縮んだ割合 f = 縮んだ行 / 書き換えた行 が `ENOMOTO_T_MIP_SPL_C` (0.6) 以上のときだけ
            // 疎化した問題を使う (kasai は 462 行が全部縮む。air03・nw04 は集合分割の 0-1 の行を書き換えるだけで 1 行も縮まず、遅くなる・メモリが増える。
            // 0.6 は f = 0.585 の physiciansched6-2 (種 1〜8 で 1.29 倍) を外す値で、MIPLIB benchmark 240 問で f が 0.585 と 0.917 の間の問題はない)。
            // 文字 `b` も付けると、書き換えた行のうち 0-1 列だけの行の割合 b が `ENOMOTO_T_MIP_SPL_BMAX` (0.5) 以下のときに限る
            // (nursesched-sprint02・decomp2・physiciansched6-2 は書き換えた行の多くが 0-1 列だけの行で、種 1〜8 で遅れた)
            let bin = |j: usize| is_int[j] && col_lo[j] == 0.0 && col_up[j] == 1.0;
            let (mut ch, mut chb, mut el) = (0usize, 0usize, 0usize);
            for (i, r) in rows.iter().enumerate() {
                if *r != prob.rows[i] {
                    ch += 1;
                    chb += prob.rows[i].iter().all(|e| bin(e.0)) as usize;
                    el += (r.iter().filter(|e| e.1 != 0.0).count() <= 1) as usize;
                }
            }
            let mode = spl_mode.unwrap_or("");
            let (f, b) = if ch > 0 { (el as f64 / ch as f64, chb as f64 / ch as f64) } else { (0.0, 0.0) };
            let use_it = !mode.contains('s')
                || (ch > 0 && f >= tunable!("ENOMOTO_T_MIP_SPL_C", 0.6, f64) && (!mode.contains('b') || b <= tunable!("ENOMOTO_T_MIP_SPL_BMAX", 0.5, f64)));
            if params.verbose {
                eprintln!("MIP: presolve: sparsified {k} rows (late)");
                let nz0: usize = prob.rows.iter().map(|r| r.len()).sum();
                let nz1: usize = rows.iter().map(|r| r.iter().filter(|e| e.1 != 0.0).count()).sum();
                let nbin = (0..prob.n).filter(|&j| bin(j)).count();
                eprintln!(
                    "MIP: sparsify-late stats m {} n {} bincols {nbin} changed {ch} binonly {chb} shrunk<=1 {el} nnz {nz0} -> {nz1} gate {:.3} (f {f:.3} b {b:.3})",
                    prob.m,
                    prob.n,
                    use_it as u8 as f64
                );
            }
            if k == 0 || !use_it {
                Some(Presolved::Reduced { prob, postsolve, scaling })
            } else {
                let q = MipProblem::from_rows(col_lo, col_up, cost, offset, sense_sign, is_int, rows, row_lo, row_up);
                SPARSIFY_REPRESOLVING.with(|f| f.set(true));
                let r = presolve_mip(&q, params.verbose, pre_submip, None);
                SPARSIFY_REPRESOLVING.with(|f| f.set(false));
                match r {
                    Some(Presolved::Infeasible) => Some(Presolved::Infeasible),
                    Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) if sc2.d.iter().chain(&scaling.d).all(|&d| d == 1.0) => {
                        let mut postsolve = postsolve;
                        postsolve.extend(post2);
                        Some(Presolved::Reduced { prob: prob2, postsolve, scaling })
                    }
                    // かけ直しが何も減らさない (または尺度が掛かった): 疎化した問題をそのまま使う
                    _ if scaling.d.iter().all(|&d| d == 1.0) => Some(Presolved::Reduced { prob: q, postsolve, scaling }),
                    _ => Some(Presolved::Reduced { prob, postsolve, scaling }),
                }
            }
        }
        other => other,
    };
    // `ENOMOTO_MIP_DUALEQ` ([`dualeq`]、2026-11-28 から既定 `1`。`=0`・`off` で使わない。analysis/presolve2_20261111.md・
    // analysis/dualeq2_20261127): 前処理の最後に、双対の議論で不等式行を
    // 等式にし ([`dual_row_equalities`])、等式にした行があれば前処理をもう一度かける (2 項等式の消去が `y_b = 1 - y_a` などを代入する)。
    // 前処理の後の問題でかけるのは、冗長な行が消え双対の固定が済んだ後で錠を数えるため。最適解の集合を狭めるだけなので後処理は要らない
    let presolved = match presolved {
        Some(Presolved::Reduced { prob, postsolve, scaling })
            if dualeq().is_some_and(|v| v != "early") && !params.submip && params.restarts == 0 && scaling.d.iter().all(|&d| d == 1.0) =>
        {
            let free_cols = |q: &MipProblem| (0..q.n).filter(|&j| q.col_lo[j] < q.col_up[j]).count();
            let MipProblem { col_lo, col_up, cost, offset, sense_sign, is_int, rows, mut row_lo, mut row_up, cols, .. } = prob.clone();
            let t0 = std::time::Instant::now();
            let k = dual_row_equalities(&rows, &mut row_lo, &mut row_up, &cols, &cost, &is_int, &col_lo, &col_up);
            // 等式にした行のうち、固定でない列が 2 本の行 (2 項等式の消去が列を代入で消せる行) の数
            let k2 = (0..rows.len())
                .filter(|&i| row_lo[i] == row_up[i] && prob.row_lo[i] != prob.row_up[i] && rows[i].iter().filter(|e| col_lo[e.0] < col_up[e.0]).count() == 2)
                .count();
            if params.verbose {
                eprintln!("MIP: presolve: DUALEQ made {k} rows equations ({k2} with two columns) (late, {:.2}s)", t0.elapsed().as_secs_f64());
            }
            // 2 列の行が 1 本もなければかけ直さない (`ENOMOTO_T_MIP_DUALEQ_NEED2=0` で数を見ない)。ns1116954 は 741 行を等式にしても
            // かけ直しが何も減らさず、かけ直し (プロービングを含む約 3 秒) の分だけ遅くなっていた
            let need2 = tunable!("ENOMOTO_T_MIP_DUALEQ_NEED2", 1usize, usize) != 0;
            if k == 0 || (need2 && k2 == 0) {
                Some(Presolved::Reduced { prob, postsolve, scaling })
            } else {
                let q = MipProblem::from_rows(col_lo, col_up, cost, offset, sense_sign, is_int, rows, row_lo, row_up);
                // 診断用 (`ENOMOTO_MIP_DEBUG_DUMP_DUALEQ=<接頭辞>`): 等式にする前 (`<接頭辞>_pre.mps`) と後 (`<接頭辞>_eq.mps`) の問題を書き出す
                if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_DUMP_DUALEQ") {
                    dump_mps(&prob, &format!("{f}_pre.mps"));
                    dump_mps(&q, &format!("{f}_eq.mps"));
                }
                match presolve_mip(&q, params.verbose, pre_submip, None) {
                    Some(Presolved::Infeasible) => Some(Presolved::Infeasible),
                    // かけ直しで行か固定でない列が減ったときだけ使う。減らなければ等式にする前の問題に戻す (等式は縮約の
                    // きっかけとしてだけ意味があり、縮まないまま等式を残すと LP の基底の引き継ぎを失い遅くなる: ns1116954 は 741 行を
                    // 等式にしても何も減らず、60 秒の回で 3〜6 秒 -> 26〜51 秒、メモリ 359 -> 2340 MB)
                    // `ENOMOTO_T_MIP_DUALEQ_MINRED` = f (試作、既定 0 = 見ない。analysis/dualeq2_20261113.md): 減った行の割合と固定でない列の
                    // 割合の大きい方が f 未満なら使わない。binkar10_1 は 822 行のうち 1 行 (C792 + C2147 ≥ 1 を等式にして連続列 1 本を代入) だけで、
                    // 等式の問題は既定でもノードが約 1.7 倍になる (カットのループの回り方は原因ではない)
                    Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 })
                        if sc2.d.iter().all(|&d| d == 1.0) && (prob2.m < prob.m || free_cols(&prob2) < free_cols(&prob)) && {
                            let f = tunable!("ENOMOTO_T_MIP_DUALEQ_MINRED", 0.0, f64);
                            let (fc, fc2) = (free_cols(&prob), free_cols(&prob2));
                            let red = ((prob.m - prob2.m.min(prob.m)) as f64 / prob.m.max(1) as f64).max((fc - fc2.min(fc)) as f64 / fc.max(1) as f64);
                            if params.verbose {
                                eprintln!("MIP: presolve: DUALEQ: presolve again: rows {} -> {}, free columns {fc} -> {fc2} (reduction {red:.4}, min {f})", prob.m, prob2.m);
                            }
                            red >= f
                        } =>
                    {
                        let mut postsolve = postsolve;
                        postsolve.extend(post2);
                        Some(Presolved::Reduced { prob: prob2, postsolve, scaling })
                    }
                    _ => {
                        if params.verbose {
                            eprintln!("MIP: presolve: DUALEQ: presolve again reduced nothing, the equations are not used");
                        }
                        Some(Presolved::Reduced { prob, postsolve, scaling })
                    }
                }
            }
        }
        other => other,
    };
    // 試作 `ENOMOTO_MIP_HNFPRE` (既定では使わない): 整数列だけの等式の Hermite 標準形 ([`hnf`])
    let presolved = match presolved {
        Some(Presolved::Reduced { prob, postsolve, scaling }) if (hnf::mode().is_some() || hnfsubst::on()) && !params.submip => {
            let _pt = pretime::guard("hnfpre");
            hnf_presolve(prob, postsolve, scaling, pre_submip)
        }
        other => other,
    };
    // 試作 `ENOMOTO_MIP_HNFSUBST` (既定では使わない): 合同式の出た一般整数列を x_j = r_j + g_j y_j に置き換える ([`hnf_subst`])。
    // `hnf_subst_map` は置き換えの一覧 (暫定解を前処理後の空間に写すのに使う。空なら置き換えなし)
    let (presolved, hnf_subst_map) = match presolved {
        Some(Presolved::Reduced { prob, postsolve, scaling }) if hnfsubst::on() && !params.submip => hnf_subst(prob, postsolve, scaling, pre_submip),
        other => (other, Vec::new()),
    };
    // 利用者向けのログ (サブ MIP の前処理は出さない)
    if !params.submip {
        let free = |q: &MipProblem| (0..q.n).filter(|&j| q.col_lo[j] < q.col_up[j]).count();
        match &presolved {
            Some(Presolved::Reduced { prob, .. }) => {
                user_log!("Presolve: {} rows, {} columns -> {} rows, {} columns ({:.2}s)", p.m, free(p), prob.m, free(prob), t_pre.elapsed().as_secs_f64())
            }
            Some(Presolved::Infeasible) => user_log!("Presolve: the problem is infeasible ({:.2}s)", t_pre.elapsed().as_secs_f64()),
            None => {}
        }
    }
    // 前処理は列の番号を保ち、列の尺度 d を掛ける (x = d x')。前処理後の空間に移して実行可能なら渡す
    // (代入で消えた列などで合わなければ渡さない)
    // `ENOMOTO_MIP_RESTART_REF` (試作): 再スタート前の暫定解 (元の行を満たす) から前処理の前後の目的値のずれを求めておき、
    // 再スタート後の求解が暫定解なしで終わっても下界を報告する
    let mut ref_shift: Option<f64> = None;
    // `ENOMOTO_MIP_TPRE`: 並列の仮の暫定値つき前処理の終わりを待ち、一番良い解を開始解に、`b` なら T 以下の解があったときに P_T の伝播の境界を持ち込む
    let mut presolved = presolved;
    let mut tpre_start = false;
    let start = match pt_job {
        Some(job) => {
            let tw = std::time::Instant::now();
            let (w, b) = job.finish(p);
            eprintln!(
                "MIP: TPRE pt: granted {:.3e} work, best {} ; waited {:.2}s at {:.2}s",
                w,
                match &b { Some((z, _, t)) => format!("{z:.10e} (thread {t})"), None => "none".into() },
                tw.elapsed().as_secs_f64(),
                t_pre.elapsed().as_secs_f64()
            );
            match (start, b) {
                (None, Some((_, x, _))) => {
                    tpre_start = true;
                    Some(x)
                }
                (s, _) => s,
            }
        }
        None => start,
    };
    let start = match tpre_job {
        // 前処理で早めの FJ が実行可能性問題の解を見つけ木に入らない道 (ns1116954) では待たない (結果は捨てる)
        Some((_, stop, _)) if FEAS_FJ_SOL.with(|s| s.borrow().is_some()) => {
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            tpre::clear_pending();
            start
        }
        // `g`: 主の手間まで許して門で止まるのを待ち、その時点の箱の解を開始解にする。以後は根と木の受け取る点で同じように読む
        Some((_, stop, _)) if tpre::mode().is_some_and(|m| m.contains('g')) => {
            let d = match &presolved {
                Some(Presolved::Reduced { scaling, .. }) => scaling.d.clone(),
                _ => vec![1.0; p.n],
            };
            let tw = std::time::Instant::now();
            let x = tpre::gated_take_orig();
            tpre::gated_set_d(d);
            eprintln!("MIP: TPRE g: at the end of the presolve: solution {} (waited {:.2}s at {:.2}s)", x.as_ref().map_or("none".to_string(), |x| format!("{:.10e}", p.objective(x))), tw.elapsed().as_secs_f64(), t_pre.elapsed().as_secs_f64());
            let _ = stop;
            match (start, x) {
                (None, Some(x)) => {
                    tpre_start = true;
                    Some(x)
                }
                (s, _) => s,
            }
        }
        // 既定は待たない (親の方針): 前処理の終わりでスレッドが終わっていなければ、主は待たずに根へ進み、スレッドは回し続ける。
        // 途中の解は箱に置かれ、主が根と木の決まった所 (`poll_lmip_thread`) で、その時点で出来上がっていれば受け取る (再現しない)。
        // `w` なら終わりを待つ (種ごとに再現するが、小さい問題で主が待つ分が目立つ: PPL で 1〜5 秒の問題が数倍遅かった)
        Some((h, stop, slot)) if !h.is_finished() && !tpre::mode().is_some_and(|m| m.contains('w')) => {
            let d = match &presolved {
                Some(Presolved::Reduced { scaling, .. }) => scaling.d.clone(),
                _ => vec![1.0; p.n],
            };
            eprintln!("MIP: TPRE: not finished at the end of the presolve ({:.2}s); the root takes its solution when ready", t_pre.elapsed().as_secs_f64());
            tpre::set_pending(slot, d, stop);
            start
        }
        Some((h, _, _)) => {
            let tw = std::time::Instant::now();
            let out = h.join().ok();
            let wait = tw.elapsed().as_secs_f64();
            let mut applied = 0usize;
            if let (Some(o), Some(Presolved::Reduced { prob, scaling, .. })) = (out.as_ref(), presolved.as_mut()) {
                if o.below_t && tpre::mode().is_some_and(|m| m.contains('b')) && prob.n == p.n && o.lo.len() == p.n {
                    for j in 0..prob.n {
                        let dj = scaling.d[j];
                        if !(dj > 0.0) || !(prob.col_lo[j] < prob.col_up[j]) {
                            continue;
                        }
                        let (mut lo, mut up) = (o.lo[j] / dj, o.up[j] / dj);
                        if prob.is_int[j] {
                            lo = (lo - 1e-6).ceil();
                            up = (up + 1e-6).floor();
                        }
                        let mut changed = false;
                        if lo > prob.col_lo[j] + 1e-9 {
                            prob.col_lo[j] = lo.min(prob.col_up[j]);
                            changed = true;
                        }
                        if up < prob.col_up[j] - 1e-9 {
                            prob.col_up[j] = up.max(prob.col_lo[j]);
                            changed = true;
                        }
                        applied += changed as usize;
                    }
                }
            }
            if let Some(o) = out.as_ref() {
                eprintln!(
                    "MIP: TPRE: L {:?} (PDLP {} iterations, {:.2}s), U {:?} ({:.2}s), T {:?}, P_T propagation: {} tightened, {} fixed, infeasible {} ({:.2}s), FJ in P_T found {} ({:.2}s), best {:.10e}; applied {applied} bounds; waited {wait:.2}s at {:.2}s",
                    o.l, o.l_iters, o.secs[0], o.u, o.secs[1], o.t, o.n_tight, o.n_fix, o.infeasible_t, o.secs[2], o.below_t, o.secs[3], o.x_z, t_pre.elapsed().as_secs_f64()
                );
            }
            tpre_start = start.is_none() && out.as_ref().is_some_and(|o| o.x.is_some());
            start.or(out.and_then(|o| o.x))
        }
        None => start,
    };
    // `ENOMOTO_MIP_TPRE` (p・pl・pt) の解: 前処理後の問題で直せなくても捨てない (下の `tpre_orig`)
    let tpre_x_orig = if tpre_start { start.clone() } else { None };
    // 試作 `ENOMOTO_MIP_VBCLQ` の `e`: 前処理の中で VB01 が見つけた解 (列の番号は前処理の前と同じ) を開始解にする (前処理後の問題で
    // 実行可能なときだけ使われる。下の確かめ)
    let vbclq_x = VBCLQ_E_SOL.with(|s| s.borrow_mut().take()).filter(|x| start.is_none() && x.len() == p.n);
    // 前処理後の問題で実行不能 (前処理の双対の簡約・対称性の固定などで外れる) でも捨てない: 元の問題の空間に戻せれば (前処理の
    // 後処理の手順を当てた点か、そのままの点が元の問題で実行可能なら) TPRE の解と同じく外側の答えの候補と打ち切り値にする (`tpre_orig`)
    let tpre_x_orig = tpre_x_orig.or_else(|| {
        let x = vbclq_x.as_ref()?;
        let via_post = match &presolved {
            Some(Presolved::Reduced { postsolve, scaling, .. }) => {
                let mut y = x.clone();
                for step in postsolve.iter().rev() {
                    step.apply(&mut y);
                }
                Some(crate::presolve::scaling::unscale_x(scaling, &y))
            }
            _ => None,
        };
        via_post.filter(|y| p.is_feasible(y, 1e-6)).or_else(|| Some(x.clone()).filter(|y| p.is_feasible(y, 1e-6)))
    });
    let start = match vbclq_x {
        Some(x) => Some(x),
        None => start,
    };
    match (&presolved, start) {
        (Some(Presolved::Reduced { prob, scaling, .. }), Some(x)) if x.len() == prob.n => {
            let mut xr: Vec<f64> = x.iter().zip(&scaling.d).map(|(v, d)| v / d).collect();
            hnfsubst::forward(&hnf_subst_map, &mut xr);
            if params.restarts > 0 && restart_ref_for(p) {
                ref_shift = Some(p.objective(&x) - prob.objective(&xr));
            }
            // `ENOMOTO_MIP_TPRE`: 元の問題の解が前処理後の問題で実行不能 (前処理の双対の固定などで外れる) なら、その点から前処理後の問題で
            // FJ を 1 回回して直す (手間は非零の数で決まる)
            let xr = if tpre_start && !prob.is_feasible(&xr, 1e-6) {
                let safety = if params.time_limit.is_finite() { 0.05 * params.time_limit } else { 10.0 };
                // 診断 `ENOMOTO_MIP_TPRE_NOREPAIR`: 直さない (直せなかった道 (`tpre_orig`) の確かめ用)
                let fixed = if env_str!("ENOMOTO_MIP_TPRE_NOREPAIR").is_some() { None } else { fj::fj2_search_from(prob, &prob.col_lo, &prob.col_up, Some(&xr), safety, 0x7E1E_5EED, None).filter(|y| prob.is_feasible(y, 1e-6)) };
                eprintln!("MIP: TPRE: the start solution is not feasible in the presolved problem; FJ from it in the presolved problem {}", match &fixed { Some(y) => format!("found {:.10e}", prob.objective(y)), None => "found nothing".into() });
                fixed.unwrap_or(xr)
            } else {
                xr
            };
            // 診断 `ENOMOTO_MIP_TPRE_FORCE_ORIG`: TPRE の解を開始解にせず、いつも外側の答えの候補と打ち切り値にする (`tpre_orig` の fuzz 用)
            let force_orig = tpre_start && env_str!("ENOMOTO_MIP_TPRE_FORCE_ORIG").is_some();
            if prob.is_feasible(&xr, 1e-6) && !force_orig {
                solver::START_SOL.with(|s| *s.borrow_mut() = Some(xr));
            } else {
                // `ENOMOTO_MIP_RESTART_REF` (試作、既定では使わない): 再スタート前の暫定解が再スタート後の問題で実行不能
                // (打ち切り値による固定で外れる) なら、暫定解にはせず RINS の参照解として渡す
                let as_ref = params.restarts > 0 && restart_ref_for(p);
                if as_ref {
                    solver::REF_SOL.with(|s| *s.borrow_mut() = Some(xr));
                }
                if params.verbose || env_str!("ENOMOTO_MIP_START_FILE").is_some() {
                    eprintln!("MIP: the start solution is not feasible in the presolved problem; dropped{}", if as_ref { " (kept as the RINS reference)" } else { "" });
                }
            }
        }
        (None, Some(x)) if x.len() == p.n => solver::START_SOL.with(|s| *s.borrow_mut() = Some(x)),
        _ => {}
    }
    // `ENOMOTO_MIP_TPRE`: 元の問題で実行可能な解 x_orig が前処理後の問題で直せなかったら、外側で答えの候補として持ち、その目的値 U を
    // 前処理後の問題の打ち切り値にする。前処理の双対の簡約は最適解を少なくとも 1 つ残し、前処理後の問題の目的値は元の問題と同じ
    // (定数項を含む。前処理は列の番号を保ち x = d x' の尺度だけ) なので、U 以上を刈っても最適解は失わない。木が U より良い解を
    // 見つけなければ x_orig を返し、「U より良い解はない」で終われば最適として返す (対称性の側の `orig_only` と同じ考え)
    // 打ち切り値 (利用者の `ENOMOTO_MIP_CUTOFF`、サブ MIP なら親の暫定解) 以上の解は予備の答えにしない: 木の打ち切り値は小さい方の打ち切り値に
    // なるので、木が示せるのは「打ち切り値より良い解はない」だけで、x_orig の値 u との間は調べていない。それでも予備の答えにすると、木が
    // 実行不能で終わったとき u を「最適」と返していた (analysis/ti6_20261124: traininstance6 で打ち切り値 28290.03 に対し VB01 の 41160 を
    // 返し、下の [`patch_user_cutoff`] の再現 (misc07・打ち切り値 1000) と同じ形)
    let tpre_orig: Option<(f64, Vec<f64>)> = tpre_x_orig
        .filter(|x| x.len() == p.n && p.is_feasible(x, 1e-6) && solver::START_SOL.with(|s| s.borrow().is_none()))
        .map(|x| (p.objective(&x), x))
        .filter(|(u, _)| !(*u >= params.cutoff - 1e-9 * params.cutoff.abs().max(1.0)));
    let params = match &tpre_orig {
        Some((u, _)) => {
            if params.verbose {
                eprintln!("MIP: TPRE: keeping the solution {u:.10e} of the original problem as the cutoff and the fallback answer");
            }
            MipParams { cutoff: params.cutoff.min(*u), ..params }
        }
        None => params,
    };
    // 結果に x_orig を当てはめる (木の解が U より悪いかなければ x_orig。実行不能・最適 (= U より良い解はない) なら最適、下界は U 以下に)
    let tpre_patch = |mut r: solver::MipResult| -> solver::MipResult {
        if let Some((u, x)) = tpre_orig.as_ref() {
            if r.objective.is_none_or(|z| z > *u + 1e-9 * u.abs().max(1.0)) || r.x.is_none() {
                r.x = Some(x.clone());
                r.objective = Some(*u);
                if matches!(r.status, MipStatus::Infeasible | MipStatus::Optimal | MipStatus::InfeasibleOrUnbounded) {
                    r.status = MipStatus::Optimal;
                    r.best_bound = *u;
                }
            }
            r.best_bound = r.best_bound.min(*u);
        }
        r
    };
    // 再スタート前のカット: 前処理は列の番号を保ち x = d x' と尺度を変えるので係数に d を掛ける。前処理で新たに
    // 固定された (消された・代入された) 列を含むカットは捨てる (代入された列の値は前処理後の境界からは分からない)
    let cuts = RESTART_CUTS.with(|s| s.borrow_mut().take());
    match (&presolved, cuts) {
        // 捨てずに後処理の手順を代入して写す (`ENOMOTO_MIP_NO_RESTART_CUTS_SUBST` で使わない) ([`map_restart_cut_subst`])
        (Some(Presolved::Reduced { prob, scaling, postsolve }), Some(cuts)) if prob.n == p.n && restart_cuts_subst() => {
            let n0 = cuts.len();
            let pairs: Vec<(usize, (Vec<(usize, f64)>, f64))> = cuts.iter().enumerate().filter_map(|(i, (c, r))| map_restart_cut_subst(c, *r, p, prob, postsolve, &scaling.d).map(|m| (i, m))).collect();
            // 診断用 (`ENOMOTO_MIP_DEBUG_RESTART_CUT_USE`): 写したカットの確かめ。前処理後の暫定解 x' での写したカットの余裕と、
            // 後処理で戻した解での元のカットの余裕は (代入が線形なので) 一致するはず
            if env_str!("ENOMOTO_MIP_DEBUG_RESTART_CUT_USE").is_some() {
                if let Some(xr) = solver::START_SOL.with(|s| s.borrow().clone()) {
                    let mut x = xr.clone();
                    for step in postsolve.iter().rev() {
                        step.apply(&mut x);
                    }
                    let x = crate::presolve::scaling::unscale_x(scaling, &x);
                    let (mut maxdiff, mut viol) = (0.0f64, 0usize);
                    for (i, (m, mr)) in &pairs {
                        let (c, r) = &cuts[*i];
                        let s0 = r - c.iter().map(|&(j, v)| v * x[j]).sum::<f64>();
                        let s1 = mr - m.iter().map(|&(j, v)| v * xr[j]).sum::<f64>();
                        maxdiff = maxdiff.max((s0 - s1).abs() / (1.0 + r.abs()));
                        viol += (s1 < -1e-6 * (1.0 + mr.abs())) as usize;
                    }
                    eprintln!("RESTART_CUT_USE: substitution check at the start solution: max slack difference {maxdiff:.3e}, mapped cuts violated {viol} of {}", pairs.len());
                }
            }
            let mapped: Vec<(Vec<(usize, f64)>, f64)> = pairs.into_iter().map(|(_, m)| m).collect();
            if params.verbose {
                let old = cuts.iter().filter(|(c, _)| c.iter().all(|&(j, _)| j < p.n && (p.col_lo[j] == p.col_up[j] || prob.col_lo[j] < prob.col_up[j]))).count();
                eprintln!("MIP: restart cuts mapped through the presolve by substitution: {} of {n0} kept (without substitution {old})", mapped.len());
            }
            RESTART_CUTS.with(|s| *s.borrow_mut() = Some(mapped));
        }
        // 代入で写さないとき (`ENOMOTO_MIP_NO_RESTART_CUTS_SUBST`)
        // 前処理で新たに固定された列を含むカットは捨てる。(試作だった `ENOMOTO_MIP_RESTART_CUTS_FIXED` (後処理が値を書き換えない固定列を
        // 含むカットは残す) は、代入で写す方がその範囲を全部含む (p200x1188c で 43/65 本 対 全部) ので消した)
        (Some(Presolved::Reduced { prob, scaling, .. }), Some(cuts)) if prob.n == p.n && hnf_subst_map.is_empty() => {
            let keep_col = |j: usize| p.col_lo[j] == p.col_up[j] || prob.col_lo[j] < prob.col_up[j];
            let mapped: Vec<(Vec<(usize, f64)>, f64)> = cuts
                .into_iter()
                .filter(|(c, _)| c.iter().all(|&(j, _)| j < p.n && keep_col(j)))
                .map(|(c, r)| (c.into_iter().map(|(j, v)| (j, v * scaling.d[j])).collect(), r))
                .collect();
            RESTART_CUTS.with(|s| *s.borrow_mut() = Some(mapped));
        }
        (None, Some(cuts)) => RESTART_CUTS.with(|s| *s.borrow_mut() = Some(cuts)),
        _ => {}
    }
    // `ENOMOTO_MIP_RESTART_PC`: 再スタート前の pseudocost を前処理の後の列へ写す (列の番号は同じ。前処理で消えた・固定された列の観測は捨てる)
    if let Some(mut pc) = RESTART_PC.with(|s| s.borrow_mut().take()) {
        let mapped = match &presolved {
            Some(Presolved::Reduced { prob, scaling, .. }) if prob.n == p.n && pc.len() == p.n => {
                let keep: Vec<bool> = (0..p.n).map(|j| prob.col_lo[j] < prob.col_up[j]).collect();
                // `ENOMOTO_MIP_HNFSUBST`: 置き換えた列は y の 1 刻みが x の g 刻み
                let mut d = scaling.d.clone();
                for s in &hnf_subst_map {
                    d[s.j] *= s.g;
                }
                pc.remap_for_restart(&keep, &d);
                Some(pc)
            }
            None if pc.len() == p.n => {
                let keep: Vec<bool> = (0..p.n).map(|j| p.col_lo[j] < p.col_up[j]).collect();
                pc.remap_for_restart(&keep, &[]);
                Some(pc)
            }
            _ => None,
        };
        RESTART_PC.with(|s| *s.borrow_mut() = mapped);
    }
    match presolved {
        Some(Presolved::Infeasible) => {
            // 診断用 (`ENOMOTO_MIP_DEBUG_DUMP_SUBINF=接頭辞`): 前処理で実行不能になったサブ MIP を前処理の前の形で `接頭辞_番号.mps` に書く
            // (本当に実行不能かを別のソルバーで確かめる。analysis/primalgap_20261121/)
            if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_DUMP_SUBINF").filter(|_| params.submip) {
                static CNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                let f = format!("{f}_{}.mps", CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
                dump_mps(p, &f);
                eprintln!("MIP: DUMP_SUBINF: the sub-MIP infeasible by presolve written to {f}");
            }
            solver::START_SOL.with(|s| *s.borrow_mut() = None);
            solver::REF_SOL.with(|s| *s.borrow_mut() = None);
            RESTART_CUTS.with(|s| *s.borrow_mut() = None);
            solver::MipResult { status: MipStatus::Infeasible, x: None, objective: None, best_bound: f64::INFINITY, nodes: 0, lp_iterations: 0 }
        }
        Some(Presolved::Reduced { mut prob, mut postsolve, scaling }) => {
            if params.submip && debug_submip_components() && heuristics::SUBMIP_DEPTH.with(|d| d.get()) == 1 {
                report_submip_components(&prob, "after presolve");
            }
            if crate::fj_p3_force() {
                let k = dual_bound_tightening(&mut prob);
                if params.verbose {
                    eprintln!("MIP: presolve: tightened {k} continuous column bounds by dual arguments");
                }
            }
            // 診断 `ENOMOTO_MIP_DEBUG_IMPLINTCOUNT` (src/mip/implint_tu.rs): 暗黙の整数の数を判定ごとに数える。`=stop` なら終える
            if let Some(v) = env_str!("ENOMOTO_MIP_DEBUG_IMPLINTCOUNT").filter(|_| !params.submip && params.restarts == 0) {
                implint_tu::debug_count(&prob, v == "stop");
            }
            // 診断 `ENOMOTO_MIP_TUBR_COUNT` (src/mip/tubr.rs、analysis/tubr_20261112.md): 分枝しなくてよい整数列 I' を数える。`=stop` なら終える
            if let Some(v) = env_str!("ENOMOTO_MIP_TUBR_COUNT").filter(|_| !params.submip && params.restarts == 0) {
                tubr::debug_count(&prob, v == "stop");
            }
            // 診断 `ENOMOTO_MIP_REFLSYM_COUNT` (src/mip/reflsym.rs、analysis/reflsym_20261112.md): 反転を含む対称性を数える。値に `stop` があれば
            // 数えた後に終える (再スタートの後も数えるなら値に `r`)
            if let Some(v) = env_str!("ENOMOTO_MIP_REFLSYM_COUNT").filter(|v| !params.submip && (params.restarts == 0 || v.contains('r'))) {
                reflsym::debug_count(&prob, &if params.restarts == 0 { "presolved".to_string() } else { format!("restart {}", params.restarts) });
                if v.contains("stop") {
                    std::process::exit(0);
                }
            }
            if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED").filter(|_| !params.submip && params.restarts == 0) {
                dump_mps(&prob, &f);
            }
            // `ENOMOTO_MIP_FEAS_FJ_EARLY`: 前処理の中の早めの FJ が解を見つけていれば、クリーク行の拡張・対称性・木を飛ばす
            let early = FEAS_FJ_SOL.with(|s| s.borrow_mut().take()).filter(|x| x.len() == prob.n && prob.is_feasible(x, 1e-6));
            let early_sol = early.is_some();
            let prob = if early_sol { prob } else if env_str!("ENOMOTO_MIP_NO_CLIQUE_EXTEND").is_none() && (!params.submip || env_str!("ENOMOTO_MIP_CLIQUE_EXTEND_SUBMIP").is_some() || submip_highs('p')) { extend_clique_rows(prob, params.verbose) } else { prob };
            // 診断用: クリーク行の拡張の後の問題 (`ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED2=ファイル`)
            if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED2").filter(|_| !params.submip && params.restarts == 0) {
                dump_mps(&prob, &f);
            }
            // 前処理が時間の上限を過ぎていれば ([`presolve_stop`]) 対称性を探さない (止めを見ず、遅い機械で 3 秒ほどかかっていた:
            // cryptanalysiskb128n5obj14、analysis/overrun_crypt_20261101.md。残り時間がないので求解はすぐ時間切れで終わる)
            let sym_late = presolve_stop();
            if sym_late && params.verbose {
                eprintln!("MIP: symmetry: skipped (the presolve passed the time limit, {:.2}s)", t_pre.elapsed().as_secs_f64());
            }
            // `ENOMOTO_MIP_HO6_SYMRESTART` (試作、既定では使わない): 再スタートの後の問題でも、それまでに対称性が見つかっていなければ
            // 探し直す ([`ho6_symrestart_due`])。HiGHS は再スタートのたびに前処理の後の模型で対称性を探す (uccase12: 元の模型には
            // 対称性がなく、根の被約費用などで固定が進んだ再スタートの後の模型で生成元 187〜262 個)。列の番号は再スタートで変わらない
            // (前処理は列を固定するだけ) ので、見つけた生成元・オービトープはそのまま次の段の求解に渡る
            let sym_restart = !early_sol && !params.submip && params.restarts > 0 && !sym_late && env_str!("ENOMOTO_MIP_NO_SYMMETRY").is_none() && ho6_symrestart_due();
            // `ENOMOTO_MIP_SUBB_SYM` (試作): サブ MIP も自分の表 ([`subb::SymScope`]) に検出する
            // 診断 `ENOMOTO_MIP_SYMPRE0_DIAG`: 前処理の後の問題で同じ数え方をする
            if env_str!("ENOMOTO_MIP_SYMPRE0_DIAG").is_some() && !params.submip && params.restarts == 0 && !early_sol {
                sympre0::diag(&prob, "after");
            }
            let prob = if !early_sol && (!params.submip || subb::sub_tables()) && params.restarts == 0 && env_str!("ENOMOTO_MIP_NO_SYMMETRY").is_none() && !sym_late {
                add_symmetry_rows(prob, params.verbose, &mut postsolve, false)
            } else if sym_restart {
                add_symmetry_rows(prob, params.verbose, &mut postsolve, true)
            } else {
                prob
            };
            // 診断用 (`ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM`、analysis/symsign_20261106.md): 最初の対称性の検出の後でプロセスを終える
            // (検出の生成元・軌道・時間だけを数える計測用)
            if !params.submip && params.restarts == 0 && env_str!("ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM").is_some() {
                eprintln!("MIP: stopped after the symmetry detection (ENOMOTO_MIP_DEBUG_STOP_AFTER_SYM, {:.2}s)", t_pre.elapsed().as_secs_f64());
                std::process::exit(0);
            }
            if !params.submip && params.restarts > 0 && !sym_restart {
                sym_of_reverify(&prob, params.verbose);
            }
            // 診断用: 対称性の行を加えた後の問題 (`ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3=ファイル`)。`ENOMOTO_MIP_DEBUG_STOP_PRESOLVED` なら
            // 書き出した後にプロセスを終える (前処理の結果の再現性の確かめ用。前処理の時間は書き出しの前に出す)
            if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3").filter(|_| !params.submip && params.restarts == 0) {
                eprintln!("MIP: presolve finished in {:.3}s (before the dump, offset {})", t_pre.elapsed().as_secs_f64(), prob.offset);
                dump_mps(&prob, &f);
                if env_str!("ENOMOTO_MIP_DEBUG_STOP_PRESOLVED").is_some() {
                    eprintln!("MIP: stopped after the presolve (ENOMOTO_MIP_DEBUG_STOP_PRESOLVED)");
                    std::process::exit(0);
                }
            }
            if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_SOL").filter(|_| !params.submip) {
                let x0: Vec<f64> = std::fs::read_to_string(f).unwrap().lines().map(|l| l.trim().parse().unwrap()).collect();
                let xd: Vec<f64> = (0..prob.n).map(|j| if prob.col_lo[j] == prob.col_up[j] { prob.col_lo[j] } else { x0[j] }).collect();
                eprintln!("MIP_DEBUG_SOL: reduced objective {} feasible {}", prob.objective(&xd), prob.is_feasible(&xd, 1e-6));
                solver::DEBUG_SOL.with(|d| *d.borrow_mut() = Some(xd));
            }
            let outer_level = !params.submip && params.restarts == 0;
            // `ENOMOTO_MIP_HNFSUBST` で置き換えたら尺度だけでは元の空間に戻せないので渡さない (カットの共有をしない)
            if outer_level && hnf_subst_map.is_empty() {
                solver::set_thread_scale(Some(scaling.d.clone()));
            }
            // `ENOMOTO_MIP_RENS_RESUME`: 一時停止できるサブ MIP なら、一時停止のときの結果を元の問題の空間に戻す手続きを置く
            let ps = std::rc::Rc::new((postsolve, scaling));
            install_resume_hook(p, params, Some(ps.clone()));
            let _mem3 = Mem3Level::push(p, &params, Some(ps.clone()));
            if HNF_PENDING.with(|f| f.replace(false)) {
                hnf_register_final(&prob, &ps.1.d);
            }
            let mut r = match early {
                // 目的関数が全部 0 なので、どの実行可能解も最適 (下界 = 目的値 = 定数)
                Some(xr) => {
                    if !params.submip && (params.restarts == 0 || pretime::restart_on()) {
                        pretime::report(prob.m, prob.n, prob.nnz, 0);
                    }
                    let z = prob.objective(&xr);
                    if params.verbose {
                        eprintln!("MIP: the early FJ solution is optimal (objective {z:e}, feasibility problem); no tree search ({:.2}s)", t_pre.elapsed().as_secs_f64());
                    }
                    solver::MipResult { status: MipStatus::Optimal, x: Some(xr), objective: Some(z), best_bound: z, nodes: 0, lp_iterations: 0 }
                }
                None => {
                    if !params.submip && (params.restarts == 0 || pretime::restart_on()) {
                        pretime::report(prob.m, prob.n, prob.nnz, (0..prob.n).filter(|&j| prob.col_lo[j] < prob.col_up[j]).count());
                    }
                    solver::solve(&prob, remaining(params))
                }
            };
            if outer_level && hnfcut::on() && env_str!("ENOMOTO_MIP_HNFPRE_QUIET").is_none() {
                eprintln!("{}", hnfcut::stats_line());
            }
            solver::RESUME_HOOK.with(|s| *s.borrow_mut() = None);
            let (postsolve, scaling) = (&ps.0, &ps.1);
            if outer_level {
                solver::set_thread_scale(None);
            }
            let mut ok = true;
            if let Some(xr) = r.x.take() {
                let xr = debug_postsolve_noise(&prob, xr);
                let mut x = xr.clone();
                for step in postsolve.iter().rev() {
                    step.apply(&mut x);
                }
                let mut x = crate::presolve::scaling::unscale_x(&scaling, &x);
                postsolve_clip(p, &mut x, params.verbose);
                if p.is_feasible(&x, 1e-6) {
                    let z_orig = p.objective(&x);
                    let shift = z_orig - prob.objective(&xr);
                    if params.verbose && shift.abs() > 1e-9 * z_orig.abs().max(1.0) {
                        eprintln!("MIP: presolve: objective shift {shift:e} between the presolved problem and the original at the incumbent (expected ~0)");
                    }
                    r.best_bound += shift;
                    r.objective = Some(z_orig);
                    r.x = Some(x);
                } else {
                    if params.verbose {
                        eprintln!("MIP: the postsolved solution is infeasible in the original problem; solving without presolve");
                    }
                    // 診断用 (`ENOMOTO_MIP_DEBUG_POSTSOLVE`): どの行・列がどれだけ破れるか、破れる行の列を後処理のどの手順が戻したか
                    if env_str!("ENOMOTO_MIP_DEBUG_POSTSOLVE").is_some() {
                        postsolve_diag(p, &prob, &xr, &x, &postsolve, &scaling.d, params.restarts);
                    }
                    // `ENOMOTO_MIP_POSTSOLVE_REPAIR` (試作、既定では使わない): 整数列を後処理した解の値に固定して `p` の LP を解き、
                    // 連続列を直す ([`repair_postsolved`])。直れば前処理なしで解き直さない
                    match repair_postsolved(p, &x, remaining(params)) {
                        Some(xn) => {
                            let z_orig = p.objective(&xn);
                            // 下界のずれは前処理後の問題と元の問題の目的値の定数の差で、後処理した (直す前の) 解 `x` で測る。
                            // 以前の式 (既定) は直した解 `xn` で測り、修復で目的値が上がった分まで下界を上げていた (根拠がなく、
                            // 木が閉じていると下界が最適値を越えうる)。`ENOMOTO_MIP_FZ3_REPAIR_BOUND` (2026-11-04 から既定、`=0` で以前の式) で
                            // `x` で測る (analysis/fuzz_tiny_classify_20261103.md)
                            let shift = if env_def!("ENOMOTO_MIP_FZ3_REPAIR_BOUND", "1").is_some() {
                                p.objective(&x) - prob.objective(&xr)
                            } else {
                                z_orig - prob.objective(&xr)
                            };
                            if params.verbose {
                                eprintln!(
                                    "MIP: postsolve repair: bound shift {shift:e} (at the postsolved point {:e}, at the repaired point {:e})",
                                    p.objective(&x) - prob.objective(&xr),
                                    z_orig - prob.objective(&xr)
                                );
                            }
                            r.best_bound += shift;
                            if env_def!("ENOMOTO_MIP_FZ3_REPAIR_BOUND", "1").is_some() {
                                // 直した解は元の問題で実行可能なので、最適値はその値以下 (下界がそれを越えたら抑える)
                                r.best_bound = r.best_bound.min(z_orig);
                            }
                            r.objective = Some(z_orig);
                            r.x = Some(xn);
                        }
                        None => ok = false,
                    }
                }
            } else if let Some(shift) = ref_shift.filter(|_| r.status != MipStatus::Infeasible) {
                r.best_bound += shift;
            }
            // 暫定解がなくても下界はそのまま報告する: 前処理は代入で生じた目的値の定数を `offset` に入れる ([`presolve_mip`]。
            // 後処理は縮約後の点についてアフィン) ので、前処理後の問題の下界は元の問題の下界でもある。以前は定数のずれが
            // 分からないとして -inf にしていた (根の下界が分かっているのに -inf を返していた: 暫定解なしの physiciansched6-2、
            // 再スタート後の求解が暫定解なしで終わる mzzv42z。analysis/bound_inf_20261021.md)
            // `ENOMOTO_MIP_LATE_NO_RESOLVE`: 時間を使い切っていれば前処理なしで解き直さない (解き直しは根の LP を解くだけで時間切れに
            // なり、時間の上限を 0.3-0.45 秒超えていた: uccase12 の再スタートの後。analysis/overrun_20261023.md)。下界はそのまま
            if !ok && r.status == MipStatus::TimeLimit && remaining(params).time_limit <= 0.0 && env_def!("ENOMOTO_MIP_LATE_NO_RESOLVE", "1").is_some() {
                if params.verbose {
                    eprintln!("MIP: no time left; not re-solving without presolve");
                }
                r.objective = None;
                if let Some(shift) = ref_shift {
                    r.best_bound += shift;
                }
                return tpre_patch(r);
            }
            if ok {
                tpre_patch(r)
            } else {
                // `ENOMOTO_MIP_RESOLVE_SYM_RESET` (2026-11-12 から既定、`=0` で以前の形): 前処理なしで解き直す前に、前処理した問題で作った
                // 対称性の控え (`SYM_ORIG` など) を消す。残すと、解き直しの木が `SYM_ORIG` (前処理した問題。列の数が `p` と同じなら
                // 見分けられない) で実行可能な点を「対称性の行にだけ反する解」として打ち切り値に使い、最後に答えとして返す
                // (`ENOMOTO_MIP_SYM_ORIG_CUT`)。前処理が誤って実行不能な問題を縮めたとき、解き直しの根の LP が実行不能と分かっても
                // その点を「最適」と返していた (analysis/objint_20261112/wrong_seed14.py、analysis/wrongans_20261112.md)
                if !params.submip && env_def!("ENOMOTO_MIP_RESOLVE_SYM_RESET", "1").is_some() {
                    clear_sym_state();
                }
                tpre_patch(solver::solve(p, remaining(params)))
            }
        }
        None => {
            if params.submip && debug_submip_components() && heuristics::SUBMIP_DEPTH.with(|d| d.get()) == 1 {
                report_submip_components(p, "after presolve (no reduction)");
            }
            if !params.submip && params.restarts > 0 {
                sym_of_reverify(p, params.verbose);
            }
            install_resume_hook(p, params, None);
            let _mem3 = Mem3Level::push(p, &params, None);
            let r = solver::solve(p, remaining(params));
            solver::RESUME_HOOK.with(|s| *s.borrow_mut() = None);
            r
        }
    }
}

/// 対称性の検出が置いたスレッドごとの控え (オービトープ・対称性の行を加える前の問題・生成元) を消す
fn clear_sym_state() {
    ORBITOPES.with(|t| *t.borrow_mut() = None);
    SYM_ORIG.with(|t| *t.borrow_mut() = None);
    SYM_LEX_GENS.with(|t| *t.borrow_mut() = None);
    SYM_SST_CHAIN.with(|t| *t.borrow_mut() = None);
    SYM_OF_GENS.with(|t| *t.borrow_mut() = None);
    symdyn::reset();
    SYM_CUT_GENS.with(|t| *t.borrow_mut() = None);
}

/// 後処理した解の列を元の問題の境界に収める (`ENOMOTO_MIP_POSTSOLVE_CLIP`、2026-11-05 から既定、`=0` で以前の形)。
/// 代入で消した列の値は後処理で行から計算し直す ([`crate::presolve::PostsolveStep::Sub`]) ので、縮約後の解が行を許容誤差の中で
/// わずかに破る (1e-8 程度) と、境界の外 (例えば下限 0 の列が -1.2e-8) になる。費用の大きい列 (binkar10_1 の費用 1e6 の罰金の列)
/// ではそれだけで目的値が 0.012 下がり、そのずれを下界にも足すので、最適値より小さい値を「最適」と返していた
/// (analysis/wrongans_20261105.md)。収めた後で元の問題の許容誤差 (1e-6) を破るなら収めない (今まで通りの解で確かめと修復に回す)
fn postsolve_clip(p: &MipProblem, x: &mut [f64], verbose: bool) {
    if env_def!("ENOMOTO_MIP_POSTSOLVE_CLIP", "1").is_none() {
        return;
    }
    let mut y = x.to_vec();
    let mut moved = 0usize;
    let mut dz = 0.0f64;
    for j in 0..p.n {
        let v = y[j].max(p.col_lo[j]).min(p.col_up[j]);
        if v != y[j] && v.is_finite() {
            dz += p.cost[j] * (v - y[j]);
            y[j] = v;
            moved += 1;
        }
    }
    if moved == 0 {
        return;
    }
    let ok = p.is_feasible(&y, 1e-6) || !p.is_feasible(x, 1e-6);
    if verbose && (dz.abs() > 1e-9 * p.objective(x).abs().max(1.0) || env_str!("ENOMOTO_MIP_DEBUG_POSTSOLVE").is_some()) {
        eprintln!("MIP: postsolve clip: {moved} columns moved into their bounds, objective change {dz:e} ({})", if ok { "used" } else { "not used: infeasible after clipping" });
    }
    if ok {
        x.copy_from_slice(&y);
    }
}

/// 診断用 (`ENOMOTO_MIP_DEBUG_POSTSOLVE_NOISE=幅`): 後処理の前に、縮約後の解の連続列へ 幅·(1+|x|) の決まった雑音を足す
/// (境界の上の列は境界の外へ、中の列は列の番号で決まる ± の向き)。
/// 縮約後の解が行をわずかに破るときに後処理がどう振る舞うかを再現する (binkar10_1 の誤答の確かめ用)
fn debug_postsolve_noise(prob: &MipProblem, mut x: Vec<f64>) -> Vec<f64> {
    let Some(w) = env_str!("ENOMOTO_MIP_DEBUG_POSTSOLVE_NOISE").and_then(|v| v.parse::<f64>().ok()) else { return x };
    for j in 0..prob.n {
        if !prob.is_int[j] {
            let sgn = if (j.wrapping_mul(2654435761) >> 7) % 2 == 0 { 1.0 } else { -1.0 };
            // 境界の上の列は外へ (許容誤差の中で境界を越える解を真似る)、中の列は ±
            let sgn = if x[j] <= prob.col_lo[j] { -1.0 } else if x[j] >= prob.col_up[j] { 1.0 } else { sgn };
            x[j] += sgn * w * (1.0 + x[j].abs());
        }
    }
    x
}

type Mem3Ps = std::rc::Rc<(Vec<crate::presolve::PostsolveStep>, crate::presolve::scaling::Scaling)>;
thread_local! {
    /// 診断用 (`ENOMOTO_MIP_MEM3_DEBUG_INC`、一時): 入れ子の求解 (最初の求解・再スタートの後の求解) ごとの (その段の前処理前の問題,
    /// 前処理の後処理の手順と尺度, 再スタートの回数)。外側から内側の順
    static MEM3_CHAIN: std::cell::RefCell<Vec<(std::rc::Rc<MipProblem>, Option<Mem3Ps>, u32)>> = const { std::cell::RefCell::new(Vec::new()) };
    /// 元の問題で書き出した暫定解の数
    static MEM3_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub(crate) fn mem3_debug_inc_on() -> bool {
    env_str!("ENOMOTO_MIP_MEM3_DEBUG_INC").is_some()
}

/// [`MEM3_CHAIN`] に段を積み、落ちるときに降ろす
struct Mem3Level(bool);
impl Mem3Level {
    fn push(p: &MipProblem, params: &MipParams, ps: Option<Mem3Ps>) -> Mem3Level {
        if params.submip || !mem3_debug_inc_on() {
            return Mem3Level(false);
        }
        MEM3_CHAIN.with(|c| c.borrow_mut().push((std::rc::Rc::new(p.clone()), ps, params.restarts)));
        Mem3Level(true)
    }
}
impl Drop for Mem3Level {
    fn drop(&mut self) {
        if self.0 {
            MEM3_CHAIN.with(|c| c.borrow_mut().pop());
        }
    }
}

/// 点 `x` の `q` での違反の要約: (最大の違反, その場所, 1e-6 (`is_feasible` と同じ許容誤差) を超える行の数, 列の境界の数, 整数性の数,
/// 違反の大きい行の上位 (行, 違反, 行の長さ))
fn mem3_viol(q: &MipProblem, x: &[f64]) -> (f64, String, usize, usize, usize, Vec<(usize, f64, usize)>) {
    let tol = |b: f64| 1e-6 + 1e-9 * b.abs();
    let (mut v, mut at) = (0.0f64, String::from("-"));
    let (mut nr, mut nc, mut ni) = (0, 0, 0);
    for j in 0..q.n {
        let e = (q.col_lo[j] - x[j]).max(x[j] - q.col_up[j]);
        if e > v {
            v = e;
            at = format!("col {j} [{}, {}] x {:.12e} int {}", q.col_lo[j], q.col_up[j], x[j], q.is_int[j]);
        }
        nc += (x[j] < q.col_lo[j] - tol(q.col_lo[j]) || x[j] > q.col_up[j] + tol(q.col_up[j])) as usize;
        ni += (q.is_int[j] && (x[j] - x[j].round()).abs() > 1e-6) as usize;
    }
    let mut top = Vec::new();
    for (i, row) in q.rows.iter().enumerate() {
        let a = exact_sum(row.iter().map(|&(j, c)| c * x[j]));
        let e = (q.row_lo[i] - a).max(a - q.row_up[i]);
        if e > v {
            v = e;
            at = format!("row {i} [{}, {}] act {:.12e} len {}", q.row_lo[i], q.row_up[i], a, row.len());
        }
        if a < q.row_lo[i] - tol(q.row_lo[i]) || a > q.row_up[i] + tol(q.row_up[i]) {
            nr += 1;
        }
        if e > 1e-7 {
            top.push((i, e, row.len()));
        }
    }
    top.sort_by(|a, b| b.1.total_cmp(&a.1));
    top.truncate(5);
    (v, at, nr, nc, ni, top)
}

/// 診断用 (`ENOMOTO_MIP_MEM3_DEBUG_INC`、一時): 木の求解 (サブ MIP でない) が暫定解 `x` (問題 `tp` の空間、目的値 `z`) を受け取ったとき、
/// 内側の段から順に後処理の手順と尺度で戻し、各段の前処理前の問題での違反と目的値を出す。元の問題まで戻した解は、値が
/// `1` でなければ `<値>_<番号>.txt` に 1 行 1 値で書く (元の問題の列の順。目的は最小化形のまま)
pub(crate) fn mem3_debug_inc(tp: &MipProblem, x: &[f64], z: f64, label: &str, nodes: u64) {
    let chain = MEM3_CHAIN.with(|c| c.borrow().clone());
    if chain.is_empty() {
        return;
    }
    let k = MEM3_COUNT.with(|c| {
        c.set(c.get() + 1);
        c.get()
    });
    let t = outer_elapsed().unwrap_or(f64::NAN);
    let (v, at, nr, nc, ni, top) = mem3_viol(tp, x);
    eprintln!("MEM3_INC #{k} [{label}] node {nodes} t {t:.2}: tree problem (restarts {}): z {z:.10e} obj {:.10e} maxviol {v:.3e} at {at}; viol rows {nr} cols {nc} int {ni}; top {top:?}", chain.last().unwrap().2, exact_objective(tp, x));
    let mut x = x.to_vec();
    for (p, ps, r) in chain.iter().rev() {
        if let Some(ps) = ps {
            for step in ps.0.iter().rev() {
                step.apply(&mut x);
            }
            x = crate::presolve::scaling::unscale_x(&ps.1, &x);
        }
        if x.len() != p.n {
            eprintln!("MEM3_INC #{k}:   level restarts {r}: length {} != n {}", x.len(), p.n);
            return;
        }
        let (v, at, nr, nc, ni, top) = mem3_viol(p, &x);
        eprintln!(
            "MEM3_INC #{k}:   level restarts {r} (before its presolve, {} rows): obj {:.10e} maxviol {v:.3e} at {at}; viol rows {nr} cols {nc} int {ni}; feasible {}; top {top:?}",
            p.m,
            exact_objective(p, &x),
            p.is_feasible(&x, 1e-6)
        );
    }
    if let Some(f) = env_str!("ENOMOTO_MIP_MEM3_DEBUG_INC").filter(|v| *v != "1") {
        let s: String = x.iter().map(|v| format!("{v:.17e}\n")).collect();
        let _ = std::fs::write(format!("{f}_{k}.txt"), s);
    }
}

/// `ENOMOTO_MIP_RENS_RESUME` ([`solver::rens_resume_on`]): このスレッドに一時停止の合図 ([`solver::RESUME_LINK`]) があれば取り出し、
/// 一時停止のときの結果 (前処理の後の問題の解) を後処理で元の問題 `p` の空間に戻してから渡す手続きを [`solver::RESUME_HOOK`] に置く
/// (次に作る求解が取り出す)。後処理した解が `p` で実行不能なら解なしとして渡す (終わりの後処理の、前処理なしの解き直しはしない)。
fn install_resume_hook(p: &MipProblem, params: MipParams, ps: Option<std::rc::Rc<(Vec<crate::presolve::PostsolveStep>, crate::presolve::scaling::Scaling)>>) {
    if !params.submip {
        return;
    }
    let Some(link) = solver::RESUME_LINK.with(|s| s.borrow_mut().take()) else { return };
    let p = p.clone();
    let hook: solver::ResumeHook = Box::new(move |mut r: solver::MipResult| {
        if let Some(xr) = r.x.take() {
            let x = match &ps {
                Some(ps) => {
                    let mut x = xr;
                    for step in ps.0.iter().rev() {
                        step.apply(&mut x);
                    }
                    crate::presolve::scaling::unscale_x(&ps.1, &x)
                }
                None => xr,
            };
            if p.is_feasible(&x, 1e-6) {
                r.objective = Some(p.objective(&x));
                r.x = Some(x);
            } else {
                r.objective = None;
            }
        }
        link.pause(r)
    });
    solver::RESUME_HOOK.with(|s| *s.borrow_mut() = Some(hook));
}

thread_local! {
    /// `ENOMOTO_MIP_FEAS_FJ_EARLY` ([`feas_fj_early_on`]): 前処理の中の早めの FJ の (スレッドの数, 時間の上限 (秒))。最初の求解の
    /// 前処理の前に置き、前処理の後に消す (サブ MIP・再スタートでは置かない)
    static FEAS_FJ_EARLY: std::cell::Cell<Option<(usize, f64)>> = const { std::cell::Cell::new(None) };
    /// 早めの FJ が見つけた解 (その時点の前処理後の問題の空間)。[`solve_problem_inner`] が取り出し、木を解かずに最適で終える
    static FEAS_FJ_SOL: std::cell::RefCell<Option<Vec<f64>>> = const { std::cell::RefCell::new(None) };
    /// 試作 `ENOMOTO_MIP_VBCLQ` の `e`: かけ直しの前処理の中 (固定の後) では、もう一度かけない
    static VBCLQ_E_ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 試作 `ENOMOTO_MIP_VBCLQ` の `e`: 前処理の中で見つけた暫定解 (列の番号は前処理の前と同じ)。前処理の後で開始解にする
    static VBCLQ_E_SOL: std::cell::RefCell<Option<Vec<f64>>> = const { std::cell::RefCell::new(None) };
}

/// `ENOMOTO_MIP_FEAS_FJ_EARLY` (既定 `fl`、`=0` で使わない。flbgx として既定にした: analysis/flbgx_default_20261102.md): 実行可能性問題 (前処理後の目的関数が全部 0) では、前処理の LP による固定
/// ([`lp_probe`]) の前に、種を変えた FJ2 をスレッドの数だけ並べて回す ([`feas_fj_early`])。見つかればどの実行可能解も最適なので、
/// 残りの前処理 (LP による固定・プロービング・クリーク行の拡張・対称性) も根の LP も飛ばして最適で終える。
/// ns1116954 (4 スレッド・60 秒) は既定で 27〜33 秒のうち、前処理の LP による固定が 11〜13 秒 (固定 0)、根の LP の解き直しが 12 秒で、
/// FJ のスレッドは根の LP の途中で解を見つけているのに主が根の LP の終わりまで受け取らない (analysis/ns1116954_20261101.md)
thread_local! {
    /// 根の緩和が非有界のときの実行可能性の問題 ([`solver::Solver::resolve_unbounded_root`]) を解いている間だけ真。早めの FJ/Local-MIP
    /// ([`feas_fj_early_on`]、時間の上限がなければ 10 秒) を飛ばす: 整数解のない問題ではそれだけ空回りしていた (slack240 で 10 秒)
    pub(crate) static UNB_CHECK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn feas_fj_early_on() -> bool {
    env_def!("ENOMOTO_MIP_FEAS_FJ_EARLY", "fl").is_some()
}

/// `ENOMOTO_MIP_FEAS_FJ_EARLY=fl` (既定): 早めの探索で FJ2 と Local-MIP を並べる ([`feas_fj_lmip_early`])。ほかの値 (`=1`) は FJ2 だけ
fn feas_fj_early_lmip() -> bool {
    env_def!("ENOMOTO_MIP_FEAS_FJ_EARLY", "fl") == Some("fl")
}

/// 早めの FJ ([`feas_fj_early_on`]) の本体: `threads` 本のスレッドで種を変えた FJ2 を `cap` 秒まで繰り返し、どれかが実行可能解を
/// 見つけたら全部止めて返す。種は FJ のスレッド ([`heuristics::fj_search`]) と重ならないようにずらす
fn feas_fj_early(prob: &MipProblem, threads: usize, cap: f64, verbose: bool) -> Option<Vec<f64>> {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    let t0 = std::time::Instant::now();
    let stop = AtomicBool::new(false);
    let tries = AtomicU64::new(0);
    let found: std::sync::Mutex<Option<Vec<f64>>> = std::sync::Mutex::new(None);
    // 前処理の期限 ([`presolve_stop`]) はスレッド局所なので、ここで読んで渡す
    let deadline = PRESOLVE_DEADLINE.with(|d| d.get());
    let base = tunable!("ENOMOTO_T_MIP_FEAS_FJ_EARLY_SEED", 0u64, u64).wrapping_mul(0xD1B5_4A32_D192_ED03) ^ 0x2545_F491_4F6C_DD1D;
    std::thread::scope(|s| {
        for t in 0..threads.max(1) {
            let (stop, tries, found) = (&stop, &tries, &found);
            s.spawn(move || {
                let mut k = 0u64;
                loop {
                    let left = cap - t0.elapsed().as_secs_f64();
                    if stop.load(Ordering::Relaxed) || left <= 0.05 || deadline.is_some_and(|d| std::time::Instant::now() >= d) {
                        break;
                    }
                    let seed = base.wrapping_add((t as u64 + k * threads.max(1) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
                    tries.fetch_add(1, Ordering::Relaxed);
                    if let Some(x) = fj::fj2_search(prob, &prob.col_lo, &prob.col_up, left, seed, Some(stop)) {
                        if prob.is_feasible(&x, 1e-6) {
                            let mut g = found.lock().unwrap();
                            if g.is_none() {
                                *g = Some(x);
                            }
                            stop.store(true, Ordering::Relaxed);
                            break;
                        }
                    }
                    k += 1;
                }
            });
        }
    });
    let r = found.into_inner().unwrap();
    if verbose {
        eprintln!(
            "MIP: presolve: early FJ for the feasibility problem: {} threads, {} tries, found {} ({:.2}s, cap {cap:.2}s)",
            threads.max(1),
            tries.load(Ordering::Relaxed),
            r.is_some(),
            t0.elapsed().as_secs_f64()
        );
    }
    r
}

/// 早めの探索の FJ2 と Local-MIP を並べる形 (`ENOMOTO_MIP_FEAS_FJ_EARLY=fl`、[`feas_fj_early_on`])。FJ が苦手で Local-MIP が得意な
/// 実行可能性問題もあるので、`threads` 本を FJ 3 対 Local-MIP 1 に分け (Local-MIP は 4 分の 1 の切り捨て、少なくとも 1 本。
/// 4 本なら FJ 3 + Local-MIP 1、8 本なら 6 + 2)、FJ 側で種を変えた FJ2 を繰り返し、残りで種を変えた Local-MIP
/// (連鎖の手あり、[`localmip::LocalMip`]) を回す。どれかが実行可能解を見つけたら止めの合図で全部止めて返す。
/// - スレッドが 1 本なら 1 本の中で交互に回す: FJ2 を 1 回、Local-MIP を `ENOMOTO_T_MIP_FEAS_EARLY_LMIP_SLICE` (0.2) 秒ずつ続きから。
/// - 時間の上限は `cap` 秒と前処理の期限 ([`PRESOLVE_DEADLINE`]) の早い方。Local-MIP は 16 手ごとにそれと止めの合図を見る。
///   Local-MIP の初期化 (行の左辺の計算・集合の行の表) は止めを見ないので、始める前に一度見る。
/// - FJ2 は止めの合図が立った後に LP 修復をしない (`fj::SKIP_REPAIR_ON_STOP`、[`heuristics::fj_lmip_mix`] と同じ)。
/// - 問題は `=1` と同じ (前処理後、LP による固定の前。FJ_P3 の前処理はかけない)。
/// `ENOMOTO_MIP_FEAS_NORETRY` (analysis/rootlp_20261120): 目的が全て 0 の問題 (実行可能性問題) で、前処理の早めの
/// FJ/Local-MIP ([`feas_fj_lmip_early`]) が解なしで上限まで回ったら、根の後のやり直し (solver の `robust_fj_retry`) を回さない。
/// fhnw-binpack4-4 (実行不能) は 4 スレッドで早めの探索 60 秒・やり直し 30 秒・木 17 秒。解ける実行可能性問題 (fhnw-binpack4-48・
/// neos-3004026-krka・ns1116954) は早めの探索が 3〜51 秒で解を見つけ、やり直しが見つけた記録はない (analysis/hang_20261117 の F)
///
/// **2026-11-20 から既定 (`=0` で以前の形、やり直しを回す)。** F の計測 (目的が全て 0 の 11 問 × 種 1〜3、NO_CHAIN と合わせて): 4 スレッド 17/17・
/// −24.9% [−39.2, −7.2]、1 スレッド 15/15・−71.2% [−92.0, −27.4]、全種で遅い問題なし (analysis/rootlp_20261120)
pub(crate) fn feas_noretry_on() -> bool {
    env_def!("ENOMOTO_MIP_FEAS_NORETRY", "1").is_some()
}

thread_local! {
    /// [`feas_noretry_on`]: この求解の早めの探索が解なしで上限まで回った (最初の求解の始めに消す)
    pub(crate) static FEAS_EARLY_DRY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn feas_fj_lmip_early(prob: &MipProblem, threads: usize, cap: f64, verbose: bool) -> Option<Vec<f64>> {
    feas_fj_lmip_early_salt(prob, threads, cap, verbose, 0)
}

/// [`feas_fj_lmip_early`] の種をずらせる形 (`salt` = 0 なら同じ)。試作 `ENOMOTO_MIP_ROBUST` の `f` (根を暫定解なしで終えるときの
/// 追加の FJ・Local-MIP、[`solver::robust`]) が、最初の早めの探索と重ならない種で回すのに使う
pub(crate) fn feas_fj_lmip_early_salt(prob: &MipProblem, threads: usize, cap: f64, verbose: bool, salt: u64) -> Option<Vec<f64>> {
    feas_fj_lmip_early_from(prob, threads, cap, verbose, salt, None, false)
}

/// [`feas_fj_lmip_early_salt`] に出発点を付けた形 (試作 `ENOMOTO_MIP_FP_STAGE3_LS`、[`heuristics`] の `fp_stage3`)。
/// `start` があれば、FJ2 は毎回その点から ([`fj::fj2_search_from`])、Local-MIP もその点から始める ([`localmip::LocalMip::set_start`]、
/// Local-MIP の中のやり直しもこの点に戻る)。`until_restart` なら、Local-MIP が自分のやり直しの判断 (改善のない手数
/// `ENOMOTO_T_MIP_LOCALMIP_RESTART`) に着いたところで止める (出発点の周りで行き詰まった印。1 本のときだけ見る)
pub(crate) fn feas_fj_lmip_early_from(prob: &MipProblem, threads: usize, cap: f64, verbose: bool, salt: u64, start: Option<&[f64]>, until_restart: bool) -> Option<Vec<f64>> {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::time::{Duration, Instant};
    let t0 = Instant::now();
    let threads = threads.max(1);
    // `ENOMOTO_T_MIP_FEAS_EARLY_LMIP_THREADS`: Local-MIP のスレッドの数 (既定は 4 分の 1 の切り捨てで少なくとも 1 本 = FJ 3 対
    // Local-MIP 1。半分ずつ (4 本で 2 + 2) は ns1116954 で 58 秒、3 + 1 は 6 秒 (analysis/feas_early_lmip_20261102.md)。
    // スレッドが 1 本なら交互)
    let n_fj = if threads == 1 { 1 } else { threads - tunable!("ENOMOTO_T_MIP_FEAS_EARLY_LMIP_THREADS", (threads / 4).max(1), usize).min(threads) };
    let stop = AtomicBool::new(false);
    let (fj_tries, lm_steps) = (AtomicU64::new(0), AtomicU64::new(0));
    // 見つけた解と、見つけた方 (1 = FJ、2 = Local-MIP) と時刻
    let found: std::sync::Mutex<Option<(Vec<f64>, u8, f64)>> = std::sync::Mutex::new(None);
    let deadline = PRESOLVE_DEADLINE.with(|d| d.get());
    // 試作 `ENOMOTO_MIP_TICKS` の `f` ([`ticks_on`]): 1 本のときは、全体の上限と Local-MIP の 1 切れを手間 ([`crate::ticks`]) で数え、
    // 時間は 10 倍の安全網にする。FJ・Local-MIP の交互の回し方が時刻によらなくなる。
    // ただし前処理の期限 (= 求解の時間の上限) は安全網の中でも守る (期限に掛かるのは時間の上限で止まるときだけなので、決定性は崩さない)。
    // 見なかった版では、根の後のやり直し ([`solver`] の robust の `g`、実行可能性問題は残りの 0.5 = cap 155 秒) の安全網が 1550 秒になり、
    // 手間の数えが実時間の約 5 分の 1 だった fhnw-binpack4-4 の 1 スレッドが時間の上限 600 秒を越えて 900 秒で kill された
    // (analysis/hang_20261117/README.md)。`ENOMOTO_MIP_FEAS_EARLY_NO_DEADLINE` で見ない (直す前と同じ、切り分け用)
    // `ENOMOTO_MIP_PAR4` の `f` (既定、[`solver::par4`]): 2 本以上でも、1 本ごとに同じ手間の期限を置く (時間は 10 倍の安全網)
    let tk_f = ticks_on('f') && (threads == 1 || solver::par4('f'));
    let t_end = if tk_f {
        let safe = t0 + Duration::from_secs_f64(10.0 * cap.max(0.0));
        match deadline {
            Some(d) if env_str!("ENOMOTO_MIP_FEAS_EARLY_NO_DEADLINE").is_none() => d.min(safe),
            _ => safe,
        }
    } else {
        deadline.map_or(t0 + Duration::from_secs_f64(cap.max(0.0)), |d| d.min(t0 + Duration::from_secs_f64(cap.max(0.0))))
    };
    let slice = tunable!("ENOMOTO_T_MIP_FEAS_EARLY_LMIP_SLICE", 0.2, f64).max(0.01);
    let base = tunable!("ENOMOTO_T_MIP_FEAS_FJ_EARLY_SEED", 0u64, u64).wrapping_mul(0xD1B5_4A32_D192_ED03) ^ 0x2545_F491_4F6C_DD1D;
    let base = if salt == 0 { base } else { base ^ salt.wrapping_mul(0x94D0_49BB_1331_11EB) };
    let done = |x: Vec<f64>, who: u8| {
        let mut g = found.lock().unwrap();
        if g.is_none() {
            *g = Some((x, who, t0.elapsed().as_secs_f64()));
        }
        stop.store(true, Ordering::Relaxed);
    };
    let over = || stop.load(Ordering::Relaxed) || Instant::now() + Duration::from_millis(50) >= t_end || (tk_f && crate::ticks::over());
    std::thread::scope(|s| {
        for t in 0..threads {
            let (fj_tries, lm_steps, done, over, stop) = (&fj_tries, &lm_steps, &done, &over, &stop);
            s.spawn(move || {
                fj::SKIP_REPAIR_ON_STOP.with(|c| c.set(true));
                let fj_seed = |k: u64| base.wrapping_add((t as u64 + k * n_fj as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
                let fj_once = |k: u64| -> bool {
                    let left = t_end.saturating_duration_since(Instant::now()).as_secs_f64();
                    fj_tries.fetch_add(1, Ordering::Relaxed);
                    match fj::fj2_search_from(prob, &prob.col_lo, &prob.col_up, start, left, fj_seed(k), Some(stop)) {
                        Some(x) if prob.is_feasible(&x, 1e-6) => {
                            done(x, 1);
                            true
                        }
                        _ => false,
                    }
                };
                let lm_seed = base ^ 0x6A09_E667_F3BC_C909 ^ ((t as u64 + 1).wrapping_mul(0xBF58_476D_1CE4_E5B9));
                // `ENOMOTO_MIP_FEAS_EARLY_LMIP_NO_CHAIN` (2026-11-20 から既定、`=0` で連鎖の手を使う以前の形。analysis/rootlp_20261120 の 1・3 節):
                // 連鎖の手を使わない。連鎖の手は 1 手が重く手間の数えが実時間の約 1/7 で、fhnw-binpack4-48 は連鎖なしの方が速く解を見つける
                // (1 スレッド 51 → 0.2 秒)。目的が全て 0 の 11 問の計測 (F、ENOMOTO_MIP_FEAS_NORETRY と合わせて) で 4 スレッド −24.9%・1 スレッド −71.2%
                let chain = env_def!("ENOMOTO_MIP_FEAS_EARLY_LMIP_NO_CHAIN", "1").is_none();
                let lm_new = || {
                    if over() {
                        return None;
                    }
                    let ti = Instant::now();
                    let mut r = localmip::LocalMip::new(prob, &prob.col_lo, &prob.col_up, lm_seed, chain);
                    if let (Some(l), Some(x)) = (r.as_mut(), start) {
                        l.set_start(x);
                    }
                    if verbose {
                        eprintln!("MIP:   early Local-MIP thread {t}: initialized in {:.2}s", ti.elapsed().as_secs_f64());
                    }
                    r
                };
                // Local-MIP を `until` まで続きから回す。見つけたら真
                let lm_run = |lm: &mut localmip::LocalMip, until: Instant| -> bool {
                    let before = lm.res.steps;
                    lm.run(until.min(t_end), Some(stop), &|_| {});
                    lm_steps.fetch_add(lm.res.steps.saturating_sub(before), Ordering::Relaxed);
                    match lm.res.best.take() {
                        Some(x) if prob.is_feasible(&x, 1e-6) => {
                            done(x, 2);
                            true
                        }
                        _ => false,
                    }
                };
                if threads == 1 {
                    // 1 本: FJ2 を 1 回と Local-MIP の 1 切れを交互に (`f` なら全体の上限は手間の期限)
                    let _tk_all = tk_f.then(|| crate::ticks::budget_guard(cap.max(0.0) * 1e9));
                    // 連鎖の手の手間も数える (数えないと Local-MIP の手間が実時間の 1/8 程度になり、上限の数倍回る。
                    // `ENOMOTO_MIP_FEAS_EARLY_NO_CHAIN_TICKS` で数えない (直す前と同じ、切り分け用))
                    let _tk_chain = (tk_f && env_str!("ENOMOTO_MIP_FEAS_EARLY_NO_CHAIN_TICKS").is_none()).then(localmip::count_chain_guard);
                    let mut lm = lm_new();
                    let mut k = 0u64;
                    // 診断 (ログ): FJ と Local-MIP の実時間と手間 ([`crate::ticks::total`]、1 手間 ≒ 1 ns のつもり) の合計
                    let mut diag = [0.0f64; 4];
                    let mut timed = |i: usize, f: &mut dyn FnMut() -> bool| -> bool {
                        let (ti, ki) = (Instant::now(), crate::ticks::total());
                        let r = f();
                        diag[i] += ti.elapsed().as_secs_f64();
                        diag[i + 1] += (crate::ticks::total() - ki) * 1e-9;
                        r
                    };
                    while !over() {
                        if timed(0, &mut || fj_once(k)) {
                            break;
                        }
                        k += 1;
                        if let Some(l) = lm.as_mut().filter(|l| !l.done) {
                            let _tk_slice = tk_f.then(|| crate::ticks::budget_guard(slice * 1e9));
                            let until = if tk_f { t_end } else { Instant::now() + Duration::from_secs_f64(slice) };
                            if !over() && timed(2, &mut || lm_run(l, until)) {
                                break;
                            }
                        }
                        if until_restart && lm.as_ref().is_none_or(|l| l.done || l.restarts() > 0) {
                            break;
                        }
                    }
                    if verbose {
                        eprintln!(
                            "MIP:   early FJ/Local-MIP (1 thread): FJ {:.2}s ({:.2}s in ticks), Local-MIP {:.2}s ({:.2}s in ticks), budget in ticks {}",
                            diag[0], diag[1], diag[2], diag[3], tk_f
                        );
                    }
                } else if t < n_fj {
                    // `PAR4` の `f`: このスレッドの手間の期限 (1 本のときと同じ上限、連鎖の手も数える)
                    let _tk_all = tk_f.then(|| crate::ticks::budget_guard(cap.max(0.0) * 1e9));
                    let mut k = 0u64;
                    while !over() {
                        if fj_once(k) {
                            break;
                        }
                        k += 1;
                    }
                } else if let Some(mut lm) = {
                    // `PAR4` の `f`: Local-MIP のスレッドも手間の期限で止める (連鎖の手の手間も数える)
                    lm_new()
                } {
                    let _tk_all = tk_f.then(|| crate::ticks::budget_guard(cap.max(0.0) * 1e9));
                    let _tk_chain = (tk_f && env_str!("ENOMOTO_MIP_FEAS_EARLY_NO_CHAIN_TICKS").is_none()).then(localmip::count_chain_guard);
                    // Local-MIP は中でやり直し (restart) をするので、上限まで 1 本を回し続ける
                    // 試作 `ENOMOTO_MIP_ROBUST` のやり直し (`salt != 0`) では `slice` ごとに見つけたかを見る。1 本で回し続けると、
                    // 途中で実行可能解を見つけても上限まで改善を続けてから返す (blend2 の 4 スレッド: 見つけたのに 6 秒使い切る)
                    let piece = |now: Instant| if salt == 0 { t_end } else { now + Duration::from_secs_f64(slice) };
                    while !over() && !lm.done {
                        if lm_run(&mut lm, piece(Instant::now())) {
                            break;
                        }
                    }
                    if verbose {
                        eprintln!(
                            "MIP:   early Local-MIP thread {t}: {} steps, least violated rows {}, chain moves {} (mean length {:.1}), {:.2}s",
                            lm.res.steps,
                            lm.res.least_viol,
                            lm.res.pair_moves,
                            lm.res.chain_len as f64 / lm.res.pair_moves.max(1) as f64,
                            t0.elapsed().as_secs_f64()
                        );
                    }
                }
                fj::SKIP_REPAIR_ON_STOP.with(|c| c.set(false));
            });
        }
    });
    let r = found.into_inner().unwrap();
    // `ENOMOTO_MIP_FEAS_NORETRY` ([`feas_noretry_on`]): 最初の早めの探索 (`salt` = 0) が解なしで上限まで回ったか
    if salt == 0 {
        FEAS_EARLY_DRY.with(|f| f.set(r.is_none()));
    }
    if verbose {
        eprintln!(
            "MIP: presolve: early FJ/Local-MIP for the feasibility problem: {threads} threads (FJ {n_fj}, Local-MIP {}), FJ {} tries, Local-MIP {} steps, found {} by {} ({:.2}s, cap {cap:.2}s)",
            if threads == 1 { 1 } else { threads - n_fj },
            fj_tries.load(Ordering::Relaxed),
            lm_steps.load(Ordering::Relaxed),
            r.is_some(),
            r.as_ref().map_or("none", |f| ["none", "FJ", "Local-MIP"][f.1 as usize]),
            r.as_ref().map_or(t0.elapsed().as_secs_f64(), |f| f.2),
        );
    }
    r.map(|f| f.0)
}

thread_local! {
    /// [`quick_probe_fixings`] の後の前処理のかけ直しの中か (早めのプロービング。かけ直しの中では行わない)。
    static QUICK_PROBE_ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// [`tighten_vubs`] の後の前処理のかけ直しの中か (`ENOMOTO_MIP_TIGHTEN_VUBS`、かけ直しの中ではもう締めない)
    static VUBS_ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// [`presolve_mip`] の 行をまとめた後のかけ直し (`ENOMOTO_MIP_NO_MERGE_REPRESOLVE` で使わない) の 2 回目の前処理の中か。
    static MERGE_REPRESOLVING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// [`presolve_mip`] の `ENOMOTO_MIP_DOMCOL` のかけ直しの中か。
    static DOMCOL_REPRESOLVING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// [`presolve_mip`] の `ENOMOTO_MIP_SPARSIFY` のかけ直しの中か。
    static SPARSIFY_REPRESOLVING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// [`probe`] が見つけた同値な 0-1 列の組 (j, k, 否定か) (`ENOMOTO_MIP_NO_PROBE_EQUIV` で使わない)。
    static PROBE_EQUIV: std::cell::RefCell<Vec<(usize, usize, bool)>> = const { std::cell::RefCell::new(Vec::new()) };
    /// 同値な列による前処理のかけ直しの深さ。
    static PROBE_EQUIV_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// [`probe`] の同値の 2 巡目 (`ENOMOTO_MIP_PROBE_EQUIV2`) が多くの組を見つけた: 境界を締めていても代入まで進む ([`probe_equiv_fix`])
    static PROBE_EQUIV_FORCE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// この求解の前処理で `ENOMOTO_MIP_PROBE_EQUIV2` の代入を使った (根のカットの `ENOMOTO_MIP_EQUIV2_CUTS` の判定用。最初の求解の始めに戻す)
    pub(crate) static EQUIV2_USED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// [`tighten_coefficients`] が許容誤差の範囲で冗長として縮めなかった係数の数 (ログ用)
    static COEF_TIGHTEN_SKIPPED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// 再スタート後の求解の前処理の間か ([`probe`] の `ENOMOTO_MIP_PROBE_EQUIV2_RESTART` などの判定)
    static RESTART_PRESOLVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

thread_local! {
    /// 今の求解 (再スタートを含む) で使う完全オービトープ ([`add_symmetry_rows`] が置き、外側の求解の終わりに消す)。
    pub(crate) static ORBITOPES: std::cell::RefCell<Option<std::rc::Rc<Vec<symmetry::Orbitope>>>> = const { std::cell::RefCell::new(None) };
    /// 対称性の行・固定を加える前の問題 (同じく外側の求解の終わりに消す)。
    pub(crate) static SYM_ORIG: std::cell::RefCell<Option<std::rc::Rc<MipProblem>>> = const { std::cell::RefCell::new(None) };
    /// lex-leader の行を作った生成元 (`ENOMOTO_MIP_SYM_LEX_CANON` でヒューリスティクスの解を写すのに使う。同じく消す)。
    pub(crate) static SYM_LEX_GENS: std::cell::RefCell<Option<std::rc::Rc<Vec<Vec<usize>>>>> = const { std::cell::RefCell::new(None) };
    /// 試作 `ENOMOTO_MIP_SYM_SST=x` ([`sym_sst_x`]): SST の行を作った段 (ヒューリスティクスの解を SST の行を満たす代表に写すのに使う。同じく消す)。
    pub(crate) static SYM_SST_CHAIN: std::cell::RefCell<Option<std::rc::Rc<Vec<symmetry::SstLevel>>>> = const { std::cell::RefCell::new(None) };
    /// orbital fixing の生成元 ([`sym_of`]。lex-leader の行の代わり。再スタートの後は [`symmetry::verify_generators`] で
    /// 確かめ直したものに置き換える。同じく消す)。
    pub(crate) static SYM_OF_GENS: std::cell::RefCell<Option<std::rc::Rc<Vec<Vec<usize>>>>> = const { std::cell::RefCell::new(None) };
    /// 根のカットの像を作る生成元 ([`sym_cuts`]。根の時点で対称性を崩す処理がないときだけ置く。同じく消す)。
    pub(crate) static SYM_CUT_GENS: std::cell::RefCell<Option<std::rc::Rc<Vec<Vec<usize>>>>> = const { std::cell::RefCell::new(None) };
}

/// `ENOMOTO_MIP_SYM_CUTS` (試作、既定では使わない): 再スタート前の根のカットを、検出した対称性の生成元で写した像もカットプールに入れ、
/// 違反していれば次のラウンドの候補にする。同じ形のブロック (箱・機械) が多数ある問題では、50 本ずつ切っても LP が切られていない
/// 別のブロックに移るだけで下界が動かないことがある (neos-3381206-awhea)。像が正しいのは、カットが対称な問題 (行・目的関数が生成元で
/// 不変) の改善解の集合で成り立つとき: 根の時点で対称性を崩す処理 (lex-leader の行・静的なオービトープの固定) が入っていなければ、
/// 根のカット (打ち切り値・被約費用による固定を使ったものも、改善解の集合が生成元で閉じているので) の像は正しい。動的な
/// orbitopal fixing・orbital fixing は分枝で固定した列から始めるので根では何も固定しない
pub(crate) fn sym_cuts() -> bool {
    env_str!("ENOMOTO_MIP_SYM_CUTS").is_some()
}

/// 試作 `ENOMOTO_MIP_SYMAVG` (analysis/symavg_20261121/、既定では使わない): 根のカットのループで、選んだカットを対称性の群で平均した 1 本
/// (列 k の係数 = k の軌道の上のカットの係数の和 / 軌道の大きさ、右辺は同じ) を足す。群の元の像の凸結合なので、像が正しい条件
/// ([`sym_cuts`] と同じ: 根の時点で対称性を崩す処理がない) の下で正しい。LP の行が群で不変なら、平均の 1 本を足した LP の値は
/// 像を全部足した LP の値と同じ (不変な LP の最適値は対称な点で取れ、対称な点ではカットとその平均の値が等しい)。
/// `=1`: 平均したカットを次のラウンドで必ず LP に入れる。`=v`: 次のラウンドで今の LP の解を破るときだけ候補にする。
pub(crate) fn sym_avg() -> Option<char> {
    env_str!("ENOMOTO_MIP_SYMAVG").and_then(|v| v.chars().next()).filter(|&c| c != '0')
}

/// 既定 (2026-10-29、`ENOMOTO_MIP_NO_SYM_OF` で以前の lex-leader の行): オービトープにならない対称性を、lex-leader の行 (`x_i - x_k >= 0`) の代わりに探索中の
/// orbital fixing (HiGHS の `StabilizerOrbits::orbitalFixing`) で扱う。行を足さないので LP が重くならず、ヒューリスティクスの
/// 解が対称性の行に反して捨てられることもない。
/// 試作 `ENOMOTO_MIP_SYM_SST`: 対称性を orbital fixing でなく Schreier-Sims の表のカットで扱う ([`symmetry::sst_pairs_except`])
pub(crate) fn sym_sst() -> bool {
    env_str!("ENOMOTO_MIP_SYM_SST").is_some_and(|v| v != "0")
}

/// `ENOMOTO_MIP_SYM_PPLEX` (analysis/scipsym_20261120/、2026-11-20 に既定にした。`=0` で以前の動き): 詰め込みの行に入る対合の多い成分を
/// 2 列の静的な詰め込みオービトープで扱う ([`symmetry::packing_orbisacks`])
pub(crate) fn sym_pplex() -> bool {
    env_def!("ENOMOTO_MIP_SYM_PPLEX", "1").is_some_and(|v| v != "0")
}

/// 試作 `ENOMOTO_MIP_SYM_SST=x` (analysis/sstx_20261119/、既定では使わない): [`sym_sst`] に加えて、ヒューリスティクスの解が SST の行だけに
/// 反するとき、元の問題 (SST の行を足す前) で実行可能なら、SST の段の群で写して行を満たす代表にしてから暫定解にする
/// ([`symmetry::sst_canonicalize`])。写せなければ目的値だけを打ち切りに使う (解は元の問題の解として持つ)。
/// SST を使う実行可能性問題 ([`sym_sst_feas`]) でも同じく写す
/// [`sym_sst_x`] の写しの回数 (外側の求解ごと、全スレッドの和): [写して SST の行を満たした, 写しても行に反した (元の x を暫定解に),
/// 写した解の確かめに落ちた (同じく元の x), 再スタートの後で写せず捨てた, 写しても今の問題で実行不能だが刈り込みの上限より悪い (捨ててよい)]。
/// 1 (写しても反した) は「今の問題 (再スタートの後の固定・カットの行を含む) で実行不能」で、SST の行に反するとは限らない。外側の求解の終わりに `MIP: SST=x:` の行で出す
pub(crate) static SST_X_STATS: [std::sync::atomic::AtomicU64; 5] = [const { std::sync::atomic::AtomicU64::new(0) }; 5];

pub(crate) fn sym_sst_x() -> bool {
    env_str!("ENOMOTO_MIP_SYM_SST").is_some_and(|v| v == "x")
}

/// `ENOMOTO_MIP_SYM_SST_FEAS` (p2inf_20261119、2026-11-19 に既定にした。`=0` で以前の動き): 目的が全部 0 の問題 (実行可能性問題) に限って [`sym_sst`] と同じく
/// SST のカットを使う。SST を orbital fixing の代わりにすると対称性のある最適化問題で遅くなる (hang_20261117 15.2・15.3 節) が、
/// fhnw-binpack4-4 (目的が全部 0) は SST で初めて実行不能を示せる。同じ成分で orbital fixing と混ぜると代表の選び方が食い違うので混ぜない
pub(crate) fn sym_sst_feas() -> bool {
    env_def!("ENOMOTO_MIP_SYM_SST_FEAS", "1").is_some_and(|v| v != "0")
}

pub(crate) fn sym_of() -> bool {
    env_str!("ENOMOTO_MIP_NO_SYM_OF").is_none()
}

/// [`sym_of`] で、再スタートの後の問題 `p` で前の求解の生成元を確かめ直し、成り立つものだけを残す。
/// `ENOMOTO_MIP_HO6_SYMRESTART` (試作): 再スタートの後に対称性を探し直すか。それまでの段で対称性 (orbital fixing の生成元・
/// lex-leader の生成元・オービトープ) が 1 つも見つかっていないときだけ
fn ho6_symrestart_due() -> bool {
    if env_str!("ENOMOTO_MIP_HO6_SYMRESTART").is_none() {
        return false;
    }
    let none = |v: Option<usize>| v.is_none_or(|k| k == 0);
    none(SYM_OF_GENS.with(|t| t.borrow().as_ref().map(|g| g.len()))) && none(SYM_LEX_GENS.with(|t| t.borrow().as_ref().map(|g| g.len()))) && none(ORBITOPES.with(|t| t.borrow().as_ref().map(|o| o.len())))
}

fn sym_of_reverify(p: &MipProblem, verbose: bool) {
    symdyn::reverify(p, verbose);
    let Some(gens) = SYM_OF_GENS.with(|t| t.borrow().clone()) else { return };
    let kept = symmetry::verify_generators(p, &gens);
    if verbose {
        eprintln!("MIP: symmetry: orbital fixing after the restart keeps {} of {} generators", kept.len(), gens.len());
    }
    SYM_OF_GENS.with(|t| *t.borrow_mut() = Some(std::rc::Rc::new(kept)));
}

/// `ENOMOTO_MIP_POSTSOLVE_REPAIR` (試作、既定では使わない): 後処理した解 `x` が `p` で実行不能なとき、整数列を `x` の値 (丸めた値) に
/// 固定した `p` の LP を解いて連続列を直す。直った解 (`p` で実行可能) を返す。整数列が境界・整数性を破っていれば試さない。
/// uccase12 では再スタート後の前処理で代入により消した連続列が、後処理で下限 0 を 1e-5 ほど破り (前処理後の問題の行の
/// 許容誤差 1e-6 が代入で拡大する。行は破れない)、11507.41 の解が捨てられていた (analysis/uccase12_20261023.md)。
/// LP の期限は残り時間と `ENOMOTO_T_MIP_POSTSOLVE_REPAIR_SECS` 秒 (既定 2 秒) の大きい方 (時間を使い切った後でも試す)
fn repair_postsolved(p: &MipProblem, x: &[f64], params: MipParams) -> Option<Vec<f64>> {
    // `params.time_limit` は残り時間
    if env_def!("ENOMOTO_MIP_POSTSOLVE_REPAIR", "1").is_none() {
        return None;
    }
    let t0 = std::time::Instant::now();
    let (mut lo, mut up) = (p.col_lo.clone(), p.col_up.clone());
    for j in 0..p.n {
        if p.is_int[j] {
            let v = x[j].round();
            if (x[j] - v).abs() > 1e-6 || v < p.col_lo[j] - 1e-6 || v > p.col_up[j] + 1e-6 {
                return None;
            }
            let v = v.clamp(p.col_lo[j], p.col_up[j]);
            lo[j] = v;
            up[j] = v;
        }
    }
    let secs = params.time_limit.max(tunable!("ENOMOTO_T_MIP_POSTSOLVE_REPAIR_SECS", 2.0, f64));
    // 時間の上限なし (`time_limit` が inf) だと `from_secs_f64` が panic する (fuzz_mixtiny の seed 58・189・230、
    // analysis/fuzz_tiny_classify_20261103.md)。`ENOMOTO_MIP_FZ3_REPAIR_BOUND` (2026-11-04 から既定、`=0` で以前の形) では期限なしにする
    let deadline = if env_def!("ENOMOTO_MIP_FZ3_REPAIR_BOUND", "1").is_some() {
        std::time::Duration::try_from_secs_f64(secs).ok().and_then(|d| t0.checked_add(d))
    } else {
        Some(t0 + std::time::Duration::from_secs_f64(secs))
    };
    // LP の前処理つきで解く (整数列を固定すると大きく縮む。前処理なしの LP は uccase12 の再スタート 1 回目の問題 (8.8 万行) で
    // 2 秒に収まらなかった。研磨 ([`polish_solution`]) と同じ経路)
    let vars: Vec<VariableData> = (0..p.n).map(|j| VariableData { vtype: VarType::Continuous, lb: lo[j], ub: up[j] }).collect();
    let cons = problem_constraints(p);
    let objective = min_objective(&p.cost, 0.0);
    let r = crate::cancel::with_deadline(deadline, || solve_lp(&vars, &objective, &cons, RootSolver::Simplex, LpOptions::default()));
    let st = r.status;
    let xn = match (&st, r.x) {
        (&Status::Optimal, Some(mut xn)) => {
            for j in 0..p.n {
                if p.is_int[j] {
                    xn[j] = lo[j];
                }
            }
            Some(xn).filter(|xn| p.is_feasible(xn, 1e-6))
        }
        _ => None,
    };
    if params.verbose {
        eprintln!(
            "MIP: postsolve repair: fixed-integer LP {st:?}, {} (objective {:.10e} -> {:?}, {:.2}s)",
            if xn.is_some() { "repaired" } else { "not repaired" },
            p.objective(x),
            xn.as_ref().map(|xn| p.objective(xn)),
            t0.elapsed().as_secs_f64()
        );
    }
    xn
}

/// 診断用 (`ENOMOTO_MIP_DEBUG_POSTSOLVE`): 後処理した解 `x` が元の問題 `p` で実行不能なとき、前処理後の解 `xr` が前処理後の
/// 問題 `prob` で実行可能か、`p` で破れる列の境界・行 (破れの大きい順に 8 本) と、破れる行の列を後処理のどの手順が戻したかを出す。
fn postsolve_diag(p: &MipProblem, prob: &MipProblem, xr: &[f64], x: &[f64], postsolve: &[crate::presolve::PostsolveStep], d: &[f64], restarts: u32) {
    use crate::presolve::PostsolveStep;
    // 列ごとに、最後に値を決めた後処理の手順 (S: 代入、P: 平行列の消された列、K: 平行列の残した列、F: 強制列。'-' は前処理後の値のまま)
    let mut kind = vec!['-'; p.n];
    // (後処理はログを逆順に当てるので、ログの前の手順ほど後に値を書く)
    for step in postsolve.iter().rev() {
        match step {
            PostsolveStep::Sub(s) => kind[s.var] = 'S',
            PostsolveStep::ParallelCol(s) => {
                kind[s.var] = 'P';
                kind[s.kept] = 'K';
            }
            PostsolveStep::Forcing(f) => kind[f.var] = 'F',
            PostsolveStep::Slack(s) => kind[s.var] = 'L',
        }
    }
    let (nsub, npar, nforce) = postsolve.iter().fold((0, 0, 0), |(a, b, c), s| match s {
        PostsolveStep::Sub(_) => (a + 1, b, c),
        PostsolveStep::ParallelCol(_) => (a, b + 1, c),
        PostsolveStep::Forcing(_) | PostsolveStep::Slack(_) => (a, b, c + 1),
    });
    eprintln!(
        "POSTSOLVE_DIAG level {restarts}: reduced solution feasible in the presolved problem {} (1e-6), {} (1e-9); steps {} (sub {nsub}, parallel {npar}, forcing {nforce})",
        prob.is_feasible(xr, 1e-6),
        prob.is_feasible(xr, 1e-9),
        postsolve.len()
    );
    let mut bad_cols: Vec<(f64, usize)> = Vec::new();
    for j in 0..p.n {
        let v = x[j];
        let viol = (p.col_lo[j] - v).max(v - p.col_up[j]).max(if p.is_int[j] { (v - v.round()).abs() } else { 0.0 });
        if !(viol <= 1e-6) {
            bad_cols.push((viol, j));
        }
    }
    bad_cols.sort_by(|a, b| b.0.total_cmp(&a.0));
    eprintln!("POSTSOLVE_DIAG bound/integrality violations {}", bad_cols.len());
    for &(viol, j) in bad_cols.iter().take(8) {
        eprintln!("POSTSOLVE_DIAG   col {j} int {} value {:.9e} bounds [{:.6e}, {:.6e}] viol {viol:.3e} step {} scale {:.4e}", p.is_int[j], x[j], p.col_lo[j], p.col_up[j], kind[j], d.get(j).copied().unwrap_or(1.0));
    }
    // 破れの大きい 3 列: 代入の式 (前処理の尺度の空間) と、式の各項が前処理後の問題の境界をどれだけ外れているか
    let mut xs = xr.to_vec();
    let mut subs: Vec<Option<&crate::presolve::colsingleton::Substitution>> = vec![None; p.n];
    for step in postsolve.iter().rev() {
        if let PostsolveStep::Sub(sub) = step {
            subs[sub.var] = Some(sub);
        }
        step.apply(&mut xs);
    }
    for &(_, j) in bad_cols.iter().take(3) {
        let Some(sub) = subs[j] else { continue };
        let mut amp = 0.0;
        let mut desc: Vec<String> = Vec::new();
        for &(k, v) in &sub.terms {
            // 前処理後の問題に残った列だけ (消した列の前処理後の境界は 0 に固定してある)
            let out = if kind[k] == '-' { (prob.col_lo[k] - xs[k]).max(xs[k] - prob.col_up[k]).max(0.0) } else { 0.0 };
            amp += (v / sub.coeff).abs() * out;
            desc.push(format!("{k}{}:{v:.4}*{:.9}[{:.4},{:.4}]{}", if p.is_int[k] { "i" } else { "" }, xs[k], prob.col_lo[k], prob.col_up[k], kind[k]));
        }
        eprintln!(
            "POSTSOLVE_DIAG   sub col {j}: scaled value {:.9e} = ({:.9e} - sum) / {:.6e}, {} terms, sum of |a_k/coeff| * (bound excess of x_k) {amp:.3e}: {}",
            xs[j],
            sub.rhs,
            sub.coeff,
            sub.terms.len(),
            desc.iter().take(16).cloned().collect::<Vec<_>>().join(" ")
        );
    }
    let mut bad_rows: Vec<(f64, usize, f64)> = Vec::new();
    for i in 0..p.m {
        let act: f64 = p.rows[i].iter().map(|&(j, a)| a * x[j]).sum();
        let viol = (p.row_lo[i] - act).max(act - p.row_up[i]);
        if !(viol <= 1e-6 + 1e-9 * p.row_lo[i].abs().max(p.row_up[i].abs()).min(1e300)) {
            bad_rows.push((viol, i, act));
        }
    }
    bad_rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    eprintln!("POSTSOLVE_DIAG row violations {}", bad_rows.len());
    for &(viol, i, act) in bad_rows.iter().take(8) {
        let cols: Vec<String> = p.rows[i].iter().map(|&(j, a)| format!("{j}{}:{a:.4}*{:.6}[{:.4},{:.4}]{}", if p.is_int[j] { "i" } else { "" }, x[j], p.col_lo[j], p.col_up[j], kind[j])).collect();
        eprintln!("POSTSOLVE_DIAG   row {i} act {act:.9e} [{:.9e}, {:.9e}] viol {viol:.3e} len {}: {}", p.row_lo[i], p.row_up[i], p.rows[i].len(), cols.iter().take(12).cloned().collect::<Vec<_>>().join(" "));
    }
}

/// `ENOMOTO_MIP_SYM_TWINS`: 対称性の検出の前に同じ列 (双子、[`symmetry::twin_classes`]) を列のハッシュで見つけ、
/// 探索ではその組を探さない。探索で見つけた生成元が動かさない双子の組は 1 列に併合し (後処理の `ParallelCol`)、動かす組
/// (より大きな対称性の一部) は隣どうしの入れ替えを生成元に足して対称性の扱いに回す。
///
/// **cand1119 (2026-11-17、analysis/treeclose_20261114.md 9.74 節) で既定にした。`0`・`off` で以前の動き (使わない)。**
pub(crate) fn sym_twins() -> bool {
    env_def!("ENOMOTO_MIP_SYM_TWINS", "1").is_some()
}

/// 既定 (2026-10-29、`ENOMOTO_MIP_NO_SYM_TWIN_ORBITOPE` で止める): 双子の組 ([`symmetry::twin_classes`]) を併合せず、探索で見つけた生成元が動かさない
/// 整数列の組を、生成元を作らずに 1xk のオービトープ (`local`、列の入れ替えが組の中だけ) として直接扱う。0-1 列のまま
/// 残り、LP の行も足さない。生成元が動かす組は [`sym_twins`] と同じく入れ替えを生成元に足す。
pub(crate) fn sym_twin_orbitope() -> bool {
    env_def!("ENOMOTO_MIP_NO_SYM_TWIN_ORBITOPE", "1").is_none()
}

/// 双子の組 `classes` をそれぞれ先頭の列に併合した問題を返し、後処理の手順を `postsolve` に足す (`parallelcols` と同じ
/// `z = x_kept + x_var`)。併合した列は境界 [0, 0]・費用 0 にして全ての行から除く。併合した列を含むプロービングの含意は捨てる。
/// [`solver::START_SOL`]・[`solver::DEBUG_SOL`] があれば同じ写しで移す。
fn merge_twin_classes(prob: MipProblem, classes: &[Vec<usize>], postsolve: &mut Vec<crate::presolve::PostsolveStep>) -> MipProblem {
    let MipProblem { mut col_lo, mut col_up, mut cost, offset, sense_sign, is_int, rows, row_lo, row_up, conflict_edges, implied_bounds, .. } = prob;
    let mut removed = vec![false; col_lo.len()];
    let mut touched = vec![false; col_lo.len()];
    let mut moves: Vec<(usize, usize)> = Vec::new();
    for c in classes {
        let kept = c[0];
        touched[kept] = true;
        let mut cur_lb = col_lo[kept];
        let mut cur_ub = col_up[kept];
        for &var in &c[1..] {
            postsolve.push(crate::presolve::PostsolveStep::ParallelCol(crate::presolve::parallelcols::Substitution { var, kept, s: 1.0, var_lb: col_lo[var], var_ub: col_up[var], kept_lb: cur_lb }));
            cur_lb += col_lo[var];
            cur_ub += col_up[var];
            removed[var] = true;
            touched[var] = true;
            col_lo[var] = 0.0;
            col_up[var] = 0.0;
            cost[var] = 0.0;
            moves.push((var, kept));
        }
        col_lo[kept] = cur_lb;
        col_up[kept] = cur_ub;
    }
    let rows: Vec<Vec<(usize, f64)>> = rows.into_iter().map(|r| r.into_iter().filter(|&(j, _)| !removed[j]).collect()).collect();
    // 併合した整数列の上限を行から締める (前処理はこの後に走らないので、集合分割の行の 0-1 列の双子が [0, 2] のまま残り、
    // 0-1 列向けの手法 (クリーク・衝突の辺) から外れていた。nw04)。行の最小・最大活動量 (無限の項の数つき) を 1 回作り、
    // 併合先の列ごとに他の項の活動量から上限を求める
    let act = |i: usize, upper: bool| -> (f64, usize) {
        let (mut v, mut inf) = (0.0, 0usize);
        for &(j, a) in &rows[i] {
            let b = if (a > 0.0) == upper { col_up[j] } else { col_lo[j] };
            if b.is_finite() { v += a * b } else { inf += 1 }
        }
        (v, inf)
    };
    let (minact, maxact): (Vec<(f64, usize)>, Vec<(f64, usize)>) = ((0..rows.len()).map(|i| act(i, false)).collect(), (0..rows.len()).map(|i| act(i, true)).collect());
    let mut kept_cols: Vec<usize> = classes.iter().map(|c| c[0]).collect();
    kept_cols.sort_unstable();
    let mut col_rows: std::collections::HashMap<usize, Vec<(usize, f64)>> = kept_cols.iter().map(|&k| (k, Vec::new())).collect();
    for (i, r) in rows.iter().enumerate() {
        for &(j, a) in r {
            if let Some(v) = col_rows.get_mut(&j) {
                v.push((i, a));
            }
        }
    }
    for &k in &kept_cols {
        if !is_int[k] {
            continue;
        }
        let mut ub = col_up[k];
        for &(i, a) in &col_rows[&k] {
            // a > 0: a z <= row_up - (他の項の最小活動量)。a < 0: a z >= row_lo - (他の項の最大活動量)
            let (bound, (v, inf), own) = if a > 0.0 { (row_up[i], minact[i], col_lo[k]) } else { (row_lo[i], maxact[i], col_lo[k]) };
            if !bound.is_finite() || !own.is_finite() || inf > 0 {
                continue;
            }
            let others = v - a * own;
            ub = ub.min(((bound - others) / a + if fz2_fix('m') { 1e-6 } else { 1e-9 }).floor());
        }
        if ub < col_up[k] && ub >= col_lo[k] {
            col_up[k] = ub;
        }
    }
    // プロービングの含意: 消した列のものは捨てる。併合先は 0-1 に戻れば残す (双子のどちらが 1 でも、もう一方と同じ含意が
    // 成り立つ)。一般整数のままなら捨てる
    let drop = |j: usize| removed[j] || (touched[j] && !(col_lo[j] == 0.0 && col_up[j] == 1.0));
    let drop: Vec<bool> = (0..removed.len()).map(drop).collect();
    let mut q = MipProblem::from_rows(col_lo, col_up, cost, offset, sense_sign, is_int, rows, row_lo, row_up);
    q.conflict_edges = conflict_edges.into_iter().filter(|&(a, b)| !drop[(a / 2) as usize] && !drop[(b / 2) as usize]).collect();
    q.implied_bounds = implied_bounds.into_iter().filter(|&(j, _, k, _, _)| !drop[j as usize] && !drop[k as usize]).collect();
    let shift = |x: &mut Vec<f64>| {
        if x.len() == removed.len() {
            for &(var, kept) in &moves {
                x[kept] += x[var];
                x[var] = 0.0;
            }
        }
    };
    solver::START_SOL.with(|t| t.borrow_mut().as_mut().map(shift));
    solver::DEBUG_SOL.with(|t| t.borrow_mut().as_mut().map(shift));
    q
}

/// 対称性を探し、対称性を崩す不等式 `x_i - x_k >= 0` ([`symmetry::lex_leader_pairs`]) を行として加えた問題を返す。
/// [`sym_twins`] で双子を併合したときは後処理の手順を `postsolve` に足す。
/// `restart` が真なら再スタートの後の探し直し (`ENOMOTO_MIP_HO6_SYMRESTART`): 時間の上限は `ENOMOTO_T_MIP_HO6_SYM_TIME` (既定 5 秒)。
/// `ENOMOTO_MIP_SUBB_SYM`: サブ MIP の検出の終わり (どの return でも) に表を数える
struct SubSymNote(std::time::Instant);

impl Drop for SubSymNote {
    fn drop(&mut self) {
        subb::note_sub_detection(self.0.elapsed().as_secs_f64());
    }
}

fn add_symmetry_rows(prob: MipProblem, verbose: bool, postsolve: &mut Vec<crate::presolve::PostsolveStep>, restart: bool) -> MipProblem {
    let _pt = pretime::guard("sym");
    let t0 = std::time::Instant::now();
    let twin_orb = sym_twin_orbitope();
    let twins = if sym_twins() || twin_orb { symmetry::twin_classes(&prob) } else { Vec::new() };
    // 既定 (`ENOMOTO_MIP_NO_SYM_TWIN_ORBITOPE` で止める) で、生成元が動かさない整数列の双子の組から作る 1xk のオービトープ
    let mut twin_orbs: Vec<symmetry::Orbitope> = Vec::new();
    let t_twins = t0.elapsed().as_secs_f64();
    // `ENOMOTO_MIP_SUBB_SYM` のサブ MIP の検出は小さい上限 (`ENOMOTO_T_MIP_SUBB_SYM_TIME`)
    let in_sub = subb::sub_tables();
    let time_cap = if in_sub {
        tunable!("ENOMOTO_T_MIP_SUBB_SYM_TIME", 0.05, f64)
    } else if restart {
        tunable!("ENOMOTO_T_MIP_HO6_SYM_TIME", 5.0, f64)
    } else {
        tunable!("ENOMOTO_T_MIP_SYM_TIME", 0.3, f64)
    };
    let _note = in_sub.then(|| SubSymNote(t0));
    if restart && verbose {
        eprintln!("MIP: symmetry: detecting again after the restart (ENOMOTO_MIP_HO6_SYMRESTART, {} rows, {} free columns)", prob.m, (0..prob.n).filter(|&j| prob.col_lo[j] < prob.col_up[j]).count());
    }
    // `ENOMOTO_MIP_PRESOLVE_WORK` (試作): 時間 (0.3 秒) の代わりに手間 (`ENOMOTO_T_MIP_PRESOLVE_WORK_SYM`、細分化で読んだ非零などの数) で打ち切る
    // 2 つ目の上限 `ENOMOTO_T_MIP_PRESOLVE_WORK_SYM_LOG` は細分化の並べ替えを L (1 + log2 L) で数えたもの ([`symmetry::log_work`])。
    // 行が少なく長い問題 (nw04: 36 行で 1 行 1.7 万の非零) では 1 つ目の数え方が細分化を軽く見て、生成元を既定 (1〜2 個) より多く (3 個)
    // 見つけ、根のカットの経路が変わって遅くなる (1 個 7〜8 秒、2 個 9.5〜10.7 秒、3 個 12.5〜13.4 秒)。0.3 秒に掛かるほかの問題では
    // 1 つ目が先に効く
    let (time_cap, work_cap, log_cap) = if presolve_work() {
        (time_cap * presolve_work_safe(), tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_SYM", 2.5e7, f64) as u64, tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_SYM_LOG", 1.3e8, f64) as u64)
    } else {
        (time_cap, u64::MAX, u64::MAX)
    };
    let (sw0, vw0, lw0, rt0) = (symmetry::search_work(), symmetry::verify_work(), symmetry::log_work(), symmetry::refine_time());
    // `ENOMOTO_MIP_HO3_SYMNORM` (2026-11-06 から既定、`=0` で止める。analysis/highs_only_20261105/G3.md): 検出に渡す問題だけ、行の向き (-1 倍) を
    // 列の順によらない形にそろえる ((下限, 上限, 係数の並べた列) と (-上限, -下限, -係数) の小さい方)。行を -1 倍しても実行可能な集合は
    // 同じなので、写しで見つけた置換は元の問題の対称性でもある。前処理は `<= 2` と `>= -2` の 2 本を 1 本の幅つきの行にまとめるが、
    // 向きが行ごとにばらばらで (fastxgemm-n2r6s0t2 では -2 <= -a-b-c+d <= 2 が 1151 本、-2 <= -a+b+c+d <= 2 が 385 本)、
    // 行を -1 倍して写す対称性が見えない (生成元 1 個、HiGHS は 39 個)
    let symnorm = env_def!("ENOMOTO_MIP_HO3_SYMNORM", "1").is_some();
    let (mut sym_caps, mut sym_time) = ((work_cap, log_cap), time_cap);
    // `ENOMOTO_MIP_SYMSIGN` (2026-11-06 から既定、`=0` で止める。analysis/symsign_20261106.md・symsign_default_20261106.md): 同点の行
    // (-1 倍しても形が同じ行) を -1 倍でも写せるようにする写し ([`symmetry::detection_copy`]。向きのそろえも含む)。
    // `ENOMOTO_MIP_SYMSCALE` (試作、既定では使わない): 行を正の倍率でそろえる。
    // `ENOMOTO_MIP_SYMZERO` (2026-11-06 から既定、`=0` で止める): 向きのそろえを、右辺 0 の等式も係数で向きを決める形 (-0.0 を 0.0 にそろえる) で行う。
    // どちらも `ENOMOTO_MIP_HO3_SYMNORM=0` (向きをそろえない) のときは既定では働かない (明示すれば働く)
    let symsign = if symnorm { env_def!("ENOMOTO_MIP_SYMSIGN", "1").is_some() } else { env_str!("ENOMOTO_MIP_SYMSIGN").is_some_and(|v| v != "0") };
    let symscale = env_str!("ENOMOTO_MIP_SYMSCALE").is_some();
    let symzero = if symnorm { env_def!("ENOMOTO_MIP_SYMZERO", "1").is_some() } else { env_str!("ENOMOTO_MIP_SYMZERO").is_some_and(|v| v != "0") };
    let normed = if symsign || symscale || symzero {
        let (q, [flipped, scaled, ties]) = symmetry::detection_copy(&prob, symscale, symsign);
        // 同点の行を二重にした分だけ確かめ ([`symmetry::detect`] の verify) の手間が増え、手間の上限に早く届いて試す組が減る
        // (neos-827175: 生成元 57 -> 46)。手間と時間の上限を写しの大きさ (非零 + 行) の比で広げ、試せる組の数を元の問題と同じくらいにする
        if ties > 0 {
            let r = (q.nnz + q.m) as f64 / (prob.nnz + prob.m).max(1) as f64;
            sym_caps = (((sym_caps.0 as f64) * r).min(u64::MAX as f64) as u64, ((sym_caps.1 as f64) * r).min(u64::MAX as f64) as u64);
            sym_time *= r;
        }
        if verbose {
            eprintln!("MIP: symmetry: SYMSIGN {symsign} SYMSCALE {symscale}: flipped {flipped}, scaled {scaled}, tie rows doubled {ties} of {} rows for the detection", prob.m);
        }
        Some(q)
    } else if symnorm {
        let mut q = prob.clone();
        let mut flipped = 0usize;
        for i in 0..q.m {
            let mut a: Vec<f64> = q.rows[i].iter().map(|t| t.1).collect();
            a.sort_by(f64::total_cmp);
            let mut b: Vec<f64> = q.rows[i].iter().map(|t| -t.1).collect();
            b.sort_by(f64::total_cmp);
            let ka = (q.row_lo[i], q.row_up[i]);
            let kb = (-q.row_up[i], -q.row_lo[i]);
            let cmp = ka.0.total_cmp(&kb.0).then(ka.1.total_cmp(&kb.1)).then_with(|| {
                for (x, y) in a.iter().zip(&b) {
                    let c = x.total_cmp(y);
                    if c != std::cmp::Ordering::Equal {
                        return c;
                    }
                }
                std::cmp::Ordering::Equal
            });
            if cmp == std::cmp::Ordering::Greater {
                flipped += 1;
                q.row_lo[i] = kb.0;
                q.row_up[i] = kb.1;
                for t in q.rows[i].iter_mut() {
                    t.1 = -t.1;
                }
                for t in q.rows[i].iter() {
                    if let Ok(k) = q.cols[t.0].binary_search_by_key(&i, |e| e.0) {
                        q.cols[t.0][k].1 = -q.cols[t.0][k].1;
                    } else if let Some(e) = q.cols[t.0].iter_mut().find(|e| e.0 == i) {
                        e.1 = -e.1;
                    }
                }
            }
        }
        if verbose {
            eprintln!("MIP: symmetry: HO3_SYMNORM flipped {flipped} of {} rows for the detection", q.m);
        }
        Some(q)
    } else {
        None
    };
    let (sym, sym_work) = sympre::detect(normed.as_ref().unwrap_or(&prob), sym_time, tunable!("ENOMOTO_T_MIP_SYM_TRIES", 2000usize, usize), sym_caps.0, sym_caps.1, &twins, verbose);
    // 診断用 (`ENOMOTO_MIP_DEBUG_DUMP_SYMGRAPH=ファイル`、analysis/sassy_20261110.md): 最初の検出に渡した問題と生成元を書き出す
    if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_DUMP_SYMGRAPH").filter(|_| !in_sub && !restart) {
        let gens: &[Vec<usize>] = sym.as_ref().map(|s| &s.generators[..]).unwrap_or(&[]);
        sympre::dump(normed.as_ref().unwrap_or(&prob), &twins, gens, t0.elapsed().as_secs_f64(), &f);
    }
    drop(normed);
    // 双子の組の扱い: 探索で見つけた生成元が動かす組は入れ替えを生成元に足し (対称性で扱う)、ほかは併合する (併合できないもの
    // (下限が無限、整数列で境界が整数でない) も入れ替えを足す)。足す入れ替えは `ENOMOTO_T_MIP_SYM_TWIN_GENS` 個 (既定 500) と、
    // 生成元の大きさ (列の数) の合計 2e7 までに抑える
    let (sym, prob) = if twins.is_empty() {
        (sym, prob)
    } else {
        let n = prob.n;
        let mut moved = vec![false; n];
        if let Some(s) = &sym {
            for g in &s.generators {
                for j in 0..n {
                    if g[j] != j {
                        moved[j] = true;
                    }
                }
            }
        }
        // 整数列の組は、併合した列の上限が行から組の列の上限まで締まる (0-1 列なら 0-1 のまま) ときだけ併合する。締まらないと
        // 一般整数列になり、0-1 列向けの手法・ヒューリスティクスが弱まる (swath1: 費用 0 で 1 行だけの 0-1 列 28 本が [0, 28] の
        // 1 列になり、根の RINS の解が 395 -> 517 と悪く、再スタートの固定が 1114 -> 1 列に減った)。行の最小活動量 (無限の項の数つき)
        // を 1 回作り、組の列の分を引いて求める
        let act_min: Vec<(f64, usize)> = (0..prob.m)
            .map(|i| {
                let (mut v, mut inf) = (0.0, 0usize);
                for &(j, a) in &prob.rows[i] {
                    let b = if a > 0.0 { prob.col_lo[j] } else { prob.col_up[j] };
                    if b.is_finite() { v += a * b } else { inf += 1 }
                }
                (v, inf)
            })
            .collect();
        let act_max: Vec<(f64, usize)> = (0..prob.m)
            .map(|i| {
                let (mut v, mut inf) = (0.0, 0usize);
                for &(j, a) in &prob.rows[i] {
                    let b = if a > 0.0 { prob.col_up[j] } else { prob.col_lo[j] };
                    if b.is_finite() { v += a * b } else { inf += 1 }
                }
                (v, inf)
            })
            .collect();
        let sum_ub = |c: &Vec<usize>| -> f64 {
            // z = 組の列の和。a > 0 の行: a z <= row_up - (組の外の最小活動量)。a < 0 の行: a z >= row_lo - (組の外の最大活動量)
            let lo = prob.col_lo[c[0]];
            let k = c.len() as f64;
            let mut ub: f64 = c.iter().map(|&j| prob.col_up[j]).sum();
            for &(i, a) in &prob.cols[c[0]] {
                // 組の列は最小活動量 (a > 0) にも最大活動量 (a < 0) にも下限で入っている
                let (bound, (v, inf)) = if a > 0.0 { (prob.row_up[i], act_min[i]) } else { (prob.row_lo[i], act_max[i]) };
                if !bound.is_finite() || !lo.is_finite() || inf > 0 {
                    continue;
                }
                // `ENOMOTO_MIP_UNIFY_FIX` の `m`: 丸めの余裕を `merge_twin_classes` (`FZ2_FIX` の `m`) と同じ 1e-6 にする
                ub = ub.min((bound - (v - a * lo * k)) / a + if solver::unify_fix('m') { 1e-6 } else { 1e-9 });
            }
            ub.floor()
        };
        let mergeable = |c: &Vec<usize>| {
            c.iter().all(|&j| prob.col_lo[j].is_finite())
                && (!prob.is_int[c[0]] || (c.iter().all(|&j| prob.col_lo[j].fract() == 0.0 && prob.col_up[j].fract() == 0.0) && sum_ub(c) <= prob.col_up[c[0]]))
        };
        let cap = tunable!("ENOMOTO_T_MIP_SYM_TWIN_GENS", 500usize, usize).min((2e7 / n.max(1) as f64) as usize);
        let (mut merge, mut keep): (Vec<Vec<usize>>, Vec<Vec<usize>>) = (Vec::new(), Vec::new());
        for c in twins {
            if twin_orb {
                // 併合せず、生成元が動かさない整数列の組は 1xk のオービトープ (連続列の組は分枝しないので扱わない)
                if c.iter().any(|&j| moved[j]) {
                    keep.push(c);
                } else if prob.is_int[c[0]] {
                    twin_orbs.push(symmetry::Orbitope { vars: vec![c.clone()], packing: false, row_packing: vec![false], swaps: Vec::new(), local: true, static_pp: false });
                }
            } else if !c.iter().any(|&j| moved[j]) && mergeable(&c) {
                merge.push(c);
            } else {
                keep.push(c);
            }
        }
        if verbose && twin_orb {
            eprintln!("MIP: symmetry: twins as orbitopes: {} classes ({} columns)", twin_orbs.len(), twin_orbs.iter().map(|o| o.vars[0].len()).sum::<usize>());
        }
        let mut gens = sym.map(|s| s.generators).unwrap_or_default();
        let mut added = 0usize;
        let mut dropped = 0usize;
        for c in &keep {
            for w in c.windows(2) {
                if added >= cap {
                    dropped += 1;
                    continue;
                }
                let mut g: Vec<usize> = (0..n).collect();
                g.swap(w[0], w[1]);
                gens.push(g);
                added += 1;
            }
        }
        let merged_cols: usize = merge.iter().map(|c| c.len() - 1).sum();
        if verbose {
            eprintln!(
                "MIP: symmetry: twins {} classes ({:.3}s): merged {} classes ({merged_cols} columns), {} classes left to the symmetry handling ({added} swaps as generators{})",
                merge.len() + keep.len(),
                t_twins,
                merge.len(),
                keep.len(),
                if dropped > 0 { format!(", {dropped} dropped by the cap") } else { String::new() }
            );
        }
        // 診断用 (`ENOMOTO_MIP_DEBUG_TWINS`): 組ごとの大きさ・整数性・境界・費用・列の非零の数・併合するか
        if env_str!("ENOMOTO_MIP_DEBUG_TWINS").is_some() {
            for (c, m) in merge.iter().map(|c| (c, true)).chain(keep.iter().map(|c| (c, false))).take(40) {
                let j = c[0];
                eprintln!("TWINS: {} cols, int {}, bounds [{}, {}], cost {}, nnz {}, merge {m}, first {:?}", c.len(), prob.is_int[j], prob.col_lo[j], prob.col_up[j], prob.cost[j], prob.cols[j].len(), &c[..c.len().min(4)]);
            }
        }
        let prob = if merge.is_empty() { prob } else { merge_twin_classes(prob, &merge, postsolve) };
        (symmetry::Symmetry::from_generators(n, gens), prob)
    };
    // 対称性の行・固定を加える前の問題 (ヒューリスティクスはこちらで解を探し、並べ替えてから使う)
    SYM_ORIG.with(|t| *t.borrow_mut() = Some(std::rc::Rc::new(prob.clone())));
    SYM_SST_CHAIN.with(|t| *t.borrow_mut() = None);
    let td = t0.elapsed().as_secs_f64();
    let sym_diag = format!("detect {td:.2}s, refine {:.2}s, log work {}", symmetry::refine_time() - rt0, symmetry::log_work() - lw0);
    // 双子のオービトープだけのとき (探索で生成元が見つからない) も、下でオービトープを置くため空の対称性で続ける
    let sym = if sym.is_none() && !twin_orbs.is_empty() { Some(symmetry::Symmetry { generators: Vec::new(), orbit: vec![usize::MAX; prob.n], num_orbits: 0 }) } else { sym };
    if verbose && (env_def!("ENOMOTO_MIP_HO6_SYMWORK", "1").is_some() || env_def!("ENOMOTO_MIP_HO6_SYMREFINE", "1").is_some()) {
        let (c, k) = symmetry::refine_capped();
        eprintln!("MIP: symmetry: refinements stopped by the iteration cap {c} of {k} (ENOMOTO_MIP_HO6_SYMREFINE {})", env_def!("ENOMOTO_MIP_HO6_SYMREFINE", "1").is_some());
    }
    // 試作 `ENOMOTO_MIP_SYMREFL` (analysis/symdyn_20261110.md): 反転を含む対称性を列を 2 倍にした写しで探す。lex-leader の行
    // (`ENOMOTO_MIP_NO_SYM_OF`) とは混ぜない。下でオービトープの列を動かすものを除いて置く
    let refl = if symdyn::symrefl() && !in_sub && sym_of() { symdyn::detect_reflections(&prob, tunable!("ENOMOTO_T_MIP_SYMREFL_TIME", 1.0, f64), verbose) } else { Vec::new() };
    let Some(sym) = sym else {
        if !refl.is_empty() {
            symdyn::set_refl(refl);
        }
        if verbose {
            eprintln!("MIP: symmetry: none ({:.2}s, work {sym_work}, search {}, verify {}, {sym_diag})", t0.elapsed().as_secs_f64(), symmetry::search_work() - sw0, symmetry::verify_work() - vw0);
        }
        return prob;
    };
    // 完全オービトープは探索中の固定 (orbitopal fixing) で扱い、残りの生成元だけ lex-leader の行にする
    let (mut orbs, mut used) = if env_str!("ENOMOTO_MIP_NO_ORBITOPE").is_none() { symmetry::find_orbitopes(&prob, &sym) } else { (Vec::new(), vec![false; sym.generators.len()]) };
    orbs.extend(twin_orbs);
    // `ENOMOTO_MIP_SYM_PPLEX` (既定、`=0` で切る。[`symmetry::packing_orbisacks`]、analysis/scipsym_20261120/): 詰め込みの行に入る対合の多い成分を、
    // 対合ごとの 2 列の静的な詰め込みオービトープ (lex-leader の orbisack) で扱う。根で各オービトープの 1 行目の右の列を 0 に固定し
    // (下の `pplex_fix`)、ORBPP_REPROBE の伝播・プロービングにかける。サブ MIP では使わない
    let mut pplex_fix: Vec<usize> = Vec::new();
    if sym_pplex() && !in_sub {
        let mut orb_moved = vec![false; prob.n];
        for o in &orbs {
            for line in &o.vars {
                for &j in line {
                    orb_moved[j] = true;
                }
            }
            for (_, _, perm) in &o.swaps {
                for (j, &t) in perm.iter().enumerate() {
                    if t != j {
                        orb_moved[j] = true;
                    }
                }
            }
        }
        let tp = std::time::Instant::now();
        let (pp, pused, st) = symmetry::packing_orbisacks(
            &prob,
            &sym,
            &used,
            &orb_moved,
            tunable!("ENOMOTO_T_MIP_SYM_PPLEX_NEW", 100usize, usize),
            tunable!("ENOMOTO_T_MIP_SYM_PPLEX_WORK", 50_000_000u64, u64),
        );
        for &k in &pused {
            used[k] = true;
        }
        pplex_fix = pp.iter().map(|o| o.vars[0][1]).collect();
        if verbose {
            let rows: usize = pp.iter().map(|o| o.vars.len()).sum();
            eprintln!(
                "MIP: symmetry: PPLEX: {} of {} components upgraded, {} packing orbisacks ({rows} rows) from {} generators and {} new involutions ({:.2}s)",
                st[1], st[0], pp.len(), pused.len(), st[2], tp.elapsed().as_secs_f64()
            );
        }
        orbs.extend(pp);
    }
    let mut in_orb = vec![false; prob.n];
    for o in &orbs {
        for line in &o.vars {
            for &j in line {
                in_orb[j] = true;
            }
        }
    }
    // 試作 `ENOMOTO_MIP_SYM_SST` ([`symmetry::sst_pairs_except`]): orbital fixing の代わりに Schreier-Sims の表のカットの行を足す
    // (SCIP 10 は fhnw-binpack4-4 をこれで 10 秒で実行不能と示す。analysis/hang_20261117/README.md)。サブ MIP では使わない
    let sst = (sym_sst() || (sym_sst_feas() && prob.cost.iter().all(|&c| c == 0.0))) && !in_sub;
    let of = sym_of() && !sst;
    let pairs = if of {
        Vec::new()
    } else if sst {
        let (pairs, levels, chain) = symmetry::sst_pairs_except(
            &sym,
            &used,
            &in_orb,
            tunable!("ENOMOTO_T_MIP_SYM_SST_GENS", 64usize, usize),
            tunable!("ENOMOTO_T_MIP_SYM_SST_WORK", 50_000_000u64, u64),
            sym_sst_x(),
        );
        if verbose {
            eprintln!("MIP: symmetry: SST cuts: {} rows in {levels} levels{}", pairs.len(), if sym_sst_x() { " (solutions mapped to the SST representative, ENOMOTO_MIP_SYM_SST=x)" } else { "" });
        }
        if !chain.is_empty() {
            SYM_SST_CHAIN.with(|t| *t.borrow_mut() = Some(std::rc::Rc::new(chain)));
        }
        pairs
    } else {
        symmetry::lex_leader_pairs_except(&sym, &used, &in_orb)
    };
    if (sym_cuts() || sym_avg().is_some()) && pairs.is_empty() && env_str!("ENOMOTO_MIP_ORBITOPE_STATIC").is_none() && !sym.generators.is_empty() {
        SYM_CUT_GENS.with(|t| *t.borrow_mut() = Some(std::rc::Rc::new(sym.generators.clone())));
    }
    if of {
        // オービトープの生成元が動かす列 (オービトープの外の列を含む) を動かす生成元は使わない (群が直積に分かれる
        // ときだけ、オービトープの固定と orbital fixing を混ぜてよい。HiGHS もオービトープの成分の置換を落とす)
        let mut orb_moved = in_orb.clone();
        for o in &orbs {
            for (_, _, perm) in &o.swaps {
                for (j, &t) in perm.iter().enumerate() {
                    if t != j {
                        orb_moved[j] = true;
                    }
                }
            }
        }
        let mut gens = symmetry::lex_leader_generators_except(&sym, &used, &orb_moved);
        // `ENOMOTO_MIP_SUBB_SYM` の `ENOMOTO_T_MIP_SUBB_SYM_OF_MAX` (既定は上限なし): サブ MIP では生成元がこの数より多ければ orbital fixing を
        // 使わない (オービトープは使う。trento1 では生成元 931 のサブ MIP の 1 ノードが約 2.5 倍遅かった)
        if in_sub && gens.len() > tunable!("ENOMOTO_T_MIP_SUBB_SYM_OF_MAX", usize::MAX, usize) {
            gens.clear();
        }
        if verbose {
            eprintln!("MIP: symmetry: orbital fixing with {} of {} generators", gens.len(), sym.generators.len());
        }
        if env_str!("ENOMOTO_MIP_SYMCOMP_DIAG").is_some() && !in_sub {
            eprintln!("MIP: symmetry: SYMCOMP {}", symdyn::component_diag(prob.n, &sym.generators, &used, &orb_moved));
        }
        SYM_OF_GENS.with(|t| *t.borrow_mut() = Some(std::rc::Rc::new(gens)));
        if !refl.is_empty() {
            let k0 = refl.len();
            let refl: Vec<symdyn::SignedGen> = refl.into_iter().filter(|s| s.moves.iter().all(|m| !orb_moved[m.0 as usize] && !orb_moved[m.1 as usize])).collect();
            if verbose {
                eprintln!("MIP: symmetry: SYMREFL keeps {} of {k0} reflection generators (not moving orbitope columns)", refl.len());
            }
            symdyn::set_refl(refl);
        }
    } else {
        SYM_LEX_GENS.with(|t| *t.borrow_mut() = Some(std::rc::Rc::new(symmetry::lex_leader_generators_except(&sym, &used, &in_orb))));
    }
    // パッキング・分割の行 (係数が全て 1、右辺 <= 1) に含まれるオービトープの行だけで作ったオービトープに置き換える
    // (列の入れ替えは全体の対称性なので、その部分行列の列を辞書式に並べた解が必ずある)。パッキングの行が半分以上の
    // ときだけ。すると列 c の最初の 1 は行 c 以降なので、x_{i,c} = 0 (c > i) を根で固定できる
    let mut orbs = orbs;
    let mut prob = prob;
    let mut static_fix = 0usize;
    for j in pplex_fix {
        if prob.col_up[j] > 0.0 && prob.col_lo[j] <= 0.0 {
            prob.col_up[j] = 0.0;
            static_fix += 1;
        }
    }
    // 既定は動的な orbitopal fixing (分枝した行をその順に使う、HiGHS と同じ): 行ごとのパッキングの印だけ付ける。
    // 根での固定 (x_{i,c} = 0, c > i) は行の固定の順に依るので、動的な固定とは混ぜられない
    let static_orbitope = env_str!("ENOMOTO_MIP_ORBITOPE_STATIC").is_some();
    for o in orbs.iter_mut() {
        let flags: Vec<bool> = o
            .vars
            .iter()
            .map(|line| {
                let set: std::collections::HashSet<usize> = line.iter().copied().collect();
                prob.cols[line[0]].iter().any(|&(i, _)| {
                    let r = &prob.rows[i];
                    prob.row_up[i] <= 1.0 + 1e-9 && r.iter().all(|&(k, a)| a == 1.0 && prob.col_lo[k] >= 0.0) && r.iter().filter(|&&(k, _)| set.contains(&k)).count() == line.len()
                })
            })
            .collect();
        o.row_packing = flags;
        o.packing = o.row_packing.iter().all(|&b| b);
    }
    if static_orbitope && env_str!("ENOMOTO_MIP_NO_PACKING_ORBITOPE").is_none() {
        for o in orbs.iter_mut() {
            let packing_rows: Vec<Vec<usize>> = o
                .vars
                .iter()
                .filter(|line| {
                    let set: std::collections::HashSet<usize> = line.iter().copied().collect();
                    let j0 = line[0];
                    prob.cols[j0].iter().any(|&(i, _)| {
                        let r = &prob.rows[i];
                        prob.row_up[i] <= 1.0 + 1e-9 && r.iter().all(|&(k, a)| a == 1.0 && prob.col_lo[k] >= 0.0) && r.iter().filter(|&&(k, _)| set.contains(&k)).count() == line.len()
                    })
                })
                .cloned()
                .collect();
            if 2 * packing_rows.len() >= o.vars.len() && !packing_rows.is_empty() {
                for (i, line) in packing_rows.iter().enumerate() {
                    for (c, &j) in line.iter().enumerate() {
                        if c > i && prob.col_up[j] > 0.0 {
                            prob.col_up[j] = 0.0;
                            static_fix += 1;
                        }
                    }
                }
                let k = packing_rows.len();
                o.vars = packing_rows;
                o.packing = true;
                o.row_packing = vec![true; k];
            }
        }
    }
    // `ENOMOTO_MIP_ORBPP` (既定で使う。`=0` で切る。analysis/vsscip_slow_20261118/g1、小さいものは下の cond): 成分ごとに扱いを選ぶ。全ての行がパッキング・分割の行に
    // 含まれ、行が 3 行以上の 0-1 のオービトープだけを静的に扱い (根で x_{i,c} = 0 (c > i) を固定し、木では全ての行を固定の順で使う)、
    // ほかは動的なまま (van Doornmalen・Hojny の 5.1 節 (c)、SCIP の orbitope_pp)。オービトープどうしは列が交わらないので、成分ごとに
    // 違う規則の代表を残してよい (群が直積に分かれる)。graph20-20-1rand で SCIP は usesymmetry=0 だと解けず、静的な固定で 2183 列が 340 列になる
    if !static_orbitope && env_def!("ENOMOTO_MIP_ORBPP", "cond").is_some() {
        let min_rows = tunable!("ENOMOTO_T_MIP_ORBPP_MIN_ROWS", 3usize, usize);
        // 列 (入れ替える組) の数の下限。列の少ないオービトープを静的にすると、固定は少ないのに動的な固定の柔らかさを失う
        // (neos-911970: 35x3 が 6 つと 35x6 が 1 つで 33 列の固定、P で 1.74 倍遅い。graph20-20-1rand は 37x37)。P・T・M の 106 問で ORBPP が
        // 働くのは 3・6 列 (neos-911970・misc07) と 37 列 (graph20) だけなので、間の 10 にする (analysis/vsscip_slow_20261118/g1/scan.txt)
        let min_cols = tunable!("ENOMOTO_T_MIP_ORBPP_MIN_COLS", 10usize, usize);
        for o in orbs.iter_mut() {
            let binary = o.vars.iter().flatten().all(|&j| prob.is_int[j] && prob.col_lo[j] >= 0.0 && prob.col_up[j] <= 1.0);
            // パッキング・分割の行に含まれる行だけのオービトープ (行の部分集合でも列の入れ替えは対称性。`ORBITOPE_STATIC` と同じく過半のときだけ)
            let rows: Vec<usize> = (0..o.vars.len()).filter(|&i| o.row_packing[i]).collect();
            if o.local || 2 * rows.len() < o.vars.len() || rows.len() < min_rows || o.vars[0].len() < min_cols || !binary {
                continue;
            }
            let swaps = std::mem::take(&mut o.swaps);
            *o = o.sub_rows(&rows);
            o.swaps = swaps;
            o.static_pp = true;
            for (i, line) in o.vars.iter().enumerate() {
                for (c, &j) in line.iter().enumerate() {
                    if c > i && prob.col_up[j] > 0.0 {
                        prob.col_up[j] = 0.0;
                        static_fix += 1;
                    }
                }
            }
        }
        if orbs.iter().any(|o| o.static_pp) {
            // 静的な固定で対称性を崩したので、根のカットの対称な像 (`ENOMOTO_MIP_SYM_CUTS`) は使えない
            SYM_CUT_GENS.with(|t| *t.borrow_mut() = None);
        }
        if verbose {
            let k = orbs.iter().filter(|o| o.static_pp).count();
            eprintln!("MIP: symmetry: ORBPP {k} of {} orbitopes static (packing, >= {min_rows} rows, >= {min_cols} columns), fixed {static_fix} columns to 0", orbs.len());
        }
    }
    if verbose && static_fix > 0 {
        eprintln!("MIP: symmetry: packing orbitopes fixed {static_fix} columns to 0");
    }
    // `ENOMOTO_MIP_ORBPP_REPROBE` (2026-11-19 から既定で使う。`=0` で使わない。analysis/scipslow_20261119 の 4・6・7 節): 静的なオービトープの固定の後に、
    // 行の伝播・オービトープの固定・プロービング (固定だけ) を繰り返し、冗長になった行を外す ([`orbpp_reprobe`])。graph20-20-1rand (4 スレッド・種 1〜5) で
    // 44.0 → 7.2 秒。静的なオービトープ (ORBPP) が働かない問題では何もしない
    if static_fix > 0 && orbs.iter().any(|o| o.static_pp) && env_def!("ENOMOTO_MIP_ORBPP_REPROBE", "1").is_some() {
        let t0 = std::time::Instant::now();
        let st = orbpp_reprobe(&mut prob, &orbs);
        if verbose {
            eprintln!(
                "MIP: symmetry: ORBPP_REPROBE: {} rounds, fixed by propagation {}, by the orbitopes {}, by probing {}, free binaries {} -> {}, redundant rows dropped {} ({:.2}s)",
                st[0] % 1000, st[1], st[2], st[3], st[4], st[5], st[0] / 1000, t0.elapsed().as_secs_f64()
            );
        }
    }
    // `ENOMOTO_MIP_ORBPP` の既定 (未設定・`=cond`、2026-11-21 から既定。analysis/orbpp_20261121): 上の扱い (MIN_COLS 列以上だけ静的) の後に、列が MIN_COLS 未満で
    // 静的の条件 (0-1・局所でない・パッキング/分割の行が過半かつ MIN_ROWS 行以上) を満たすオービトープを写しの問題で静的に固定し、REPROBE をかける。
    // 比べのために、静的にしない写し (既定の問題そのものの写し) にも同じ REPROBE をかける (前の REPROBE が回数・手間の上限で残した固定や、オービトープに
    // よらない伝播・プロービングの固定を差し引くため)。静的にした写しで固定され、静的にしない写しでは固定されない列 (静的固定そのものは除く) が
    // 1 列でもあればその写しを採り、0 なら写しを捨てて上の扱いのままの問題・オービトープ (動的) に戻す。`=1` (など cond 以外の値) で以前の既定
    // (MIN_COLS 列以上だけ静的、小さいものは動的)、`=0` で ORBPP を全て使わない。P (60 秒・種 1〜3) 0.997 [0.987, 1.008]・M (600 秒) 0.998 [0.973, 1.023]
    // 閾値は置かない (固定が増えるか否か)。neos-911970 (固定 0、遅くなる) と misc07 (45 列、2.3 倍速い) を分ける
    if !static_orbitope && env_def!("ENOMOTO_MIP_ORBPP", "cond") == Some("cond") && env_def!("ENOMOTO_MIP_ORBPP_REPROBE", "1").is_some() {
        let min_rows = tunable!("ENOMOTO_T_MIP_ORBPP_MIN_ROWS", 3usize, usize);
        let min_cols = tunable!("ENOMOTO_T_MIP_ORBPP_MIN_COLS", 10usize, usize);
        let mut q = prob.clone();
        let mut o2 = orbs.clone();
        let mut add = 0usize;
        let mut sfix = vec![false; q.n];
        let mut k_new = 0usize;
        for o in o2.iter_mut() {
            if o.static_pp || o.vars[0].len() >= min_cols {
                continue;
            }
            let binary = o.vars.iter().flatten().all(|&j| q.is_int[j] && q.col_lo[j] >= 0.0 && q.col_up[j] <= 1.0);
            let rows: Vec<usize> = (0..o.vars.len()).filter(|&i| o.row_packing[i]).collect();
            if o.local || 2 * rows.len() < o.vars.len() || rows.len() < min_rows || !binary {
                continue;
            }
            let swaps = std::mem::take(&mut o.swaps);
            *o = o.sub_rows(&rows);
            o.swaps = swaps;
            o.static_pp = true;
            k_new += 1;
            for (i, line) in o.vars.iter().enumerate() {
                for (c, &j) in line.iter().enumerate() {
                    if c > i && q.col_up[j] > 0.0 {
                        q.col_up[j] = 0.0;
                        sfix[j] = true;
                        add += 1;
                    }
                }
            }
        }
        if k_new > 0 {
            let t0 = std::time::Instant::now();
            let mut q0 = prob.clone();
            let st0 = orbpp_reprobe(&mut q0, &orbs);
            let st = orbpp_reprobe(&mut q, &o2);
            let fixed = |p: &MipProblem, j: usize| p.col_lo[j] == p.col_up[j];
            let new = (0..q.n).filter(|&j| q.is_int[j] && !sfix[j] && fixed(&q, j) && !fixed(&q0, j)).count();
            if verbose {
                eprintln!(
                    "MIP: symmetry: ORBPP cond: {k_new} small orbitopes, static fixed {add}, REPROBE fixed {} (propagation {}, orbitopes {}, probing {}), without them {}, new {new} -> {} ({:.2}s)",
                    st[1] + st[2] + st[3],
                    st[1],
                    st[2],
                    st[3],
                    st0[1] + st0[2] + st0[3],
                    if new > 0 { "static" } else { "reverted to dynamic" },
                    t0.elapsed().as_secs_f64()
                );
            }
            if new > 0 {
                prob = q;
                orbs = o2;
                SYM_CUT_GENS.with(|t| *t.borrow_mut() = None);
            }
        }
    }
    if env_str!("ENOMOTO_MIP_SYM_DEBUG").is_some() {
        // オービトープの各行に、その行の列だけからなる (他の列も含んでよい) 集合分割・パッキング行があるか
        for o in &orbs {
            let mut packing = 0;
            for line in &o.vars {
                let set: std::collections::HashSet<usize> = line.iter().copied().collect();
                let has = prob.rows.iter().enumerate().any(|(i, r)| prob.row_up[i] == 1.0 && r.iter().all(|&(_, a)| a == 1.0) && line.iter().all(|j| r.iter().any(|&(k, _)| k == *j)) && r.iter().filter(|&&(k, _)| set.contains(&k)).count() == line.len());
                packing += has as usize;
            }
            eprintln!("SYM orbitope {}x{}: {} of {} rows lie in a packing/partitioning row", o.vars.len(), o.vars[0].len(), packing, o.vars.len());
        }
    }
    if verbose && !orbs.is_empty() {
        let dims: Vec<String> = orbs.iter().map(|o| format!("{}x{}{}", o.vars.len(), o.vars[0].len(), if o.packing { " packing" } else { "" })).collect();
        eprintln!("MIP: symmetry: {} full orbitopes ({})", orbs.len(), dims.join(", "));
    }
    ORBITOPES.with(|t| *t.borrow_mut() = Some(std::rc::Rc::new(orbs)));
    if env_str!("ENOMOTO_MIP_SYM_DEBUG").is_some() {
        for (k, g) in sym.generators.iter().enumerate() {
            // 巡回の長さの分布
            let mut seen = vec![false; g.len()];
            let mut lens: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
            for j in 0..g.len() {
                if seen[j] || g[j] == j {
                    continue;
                }
                let mut l = 0;
                let mut x = j;
                while !seen[x] {
                    seen[x] = true;
                    x = g[x];
                    l += 1;
                }
                *lens.entry(l).or_insert(0) += 1;
            }
            let moved: Vec<usize> = (0..g.len()).filter(|&j| g[j] != j).take(12).collect();
            eprintln!("SYM gen {k}: cycles {lens:?} first moved {moved:?} -> {:?}", moved.iter().map(|&j| g[j]).collect::<Vec<_>>());
        }
    }
    if verbose {
        let moved = sym.orbit.iter().filter(|&&o| o != usize::MAX).count();
        eprintln!("MIP: symmetry: {} generators, {} orbits on {moved} columns, {} lex-leader rows ({:.2}s, work {sym_work}, search {}, verify {}, {sym_diag})", sym.generators.len(), sym.num_orbits, pairs.len(), t0.elapsed().as_secs_f64(), symmetry::search_work() - sw0, symmetry::verify_work() - vw0);
    }
    if pairs.is_empty() || env_str!("ENOMOTO_MIP_SYM_DETECT_ONLY").is_some() {
        return prob;
    }
    let MipProblem { col_lo, col_up, cost, offset, sense_sign, is_int, mut rows, mut row_lo, mut row_up, conflict_edges, implied_bounds, .. } = prob;
    for (i, k) in pairs {
        rows.push(vec![(i.min(k), if i < k { 1.0 } else { -1.0 }), (i.max(k), if i < k { -1.0 } else { 1.0 })]);
        row_lo.push(0.0);
        row_up.push(f64::INFINITY);
    }
    // プロービングの含意は行を足しても成り立つので引き継ぐ (以前は落としていた)
    let mut q = MipProblem::from_rows(col_lo, col_up, cost, offset, sense_sign, is_int, rows, row_lo, row_up);
    q.conflict_edges = conflict_edges;
    q.implied_bounds = implied_bounds;
    q
}

/// 診断用 (`ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED=ファイル`): 前処理後の問題 (最小化形、offset は目的の定数列 `OBJCONST` に入れない) を
/// 固定形式でない MPS に書き出す。列は `C<番号>`、行は `R<番号>`。
pub(crate) fn dump_mps(p: &MipProblem, f: &str) {
    use std::fmt::Write as _;
    let mut s = String::new();
    let _ = writeln!(s, "NAME presolved\nROWS\n N OBJ");
    for i in 0..p.m {
        let t = if p.row_lo[i] == p.row_up[i] { "E" } else if p.row_up[i].is_finite() { "L" } else { "G" };
        let _ = writeln!(s, " {t} R{i}");
    }
    let _ = writeln!(s, "COLUMNS");
    let mut in_int = false;
    for j in 0..p.n {
        if p.is_int[j] != in_int {
            let _ = writeln!(s, "    M{j} 'MARKER' {}", if p.is_int[j] { "'INTORG'" } else { "'INTEND'" });
            in_int = p.is_int[j];
        }
        if p.cost[j] != 0.0 {
            let _ = writeln!(s, "    C{j} OBJ {:e}", p.cost[j]);
        }
        for &(i, a) in &p.cols[j] {
            let _ = writeln!(s, "    C{j} R{i} {a:e}");
        }
        if p.cost[j] == 0.0 && p.cols[j].is_empty() {
            let _ = writeln!(s, "    C{j} OBJ 0");
        }
    }
    if in_int {
        let _ = writeln!(s, "    MEND 'MARKER' 'INTEND'");
    }
    let _ = writeln!(s, "RHS");
    for i in 0..p.m {
        let r = if p.row_up[i].is_finite() { p.row_up[i] } else { p.row_lo[i] };
        if r != 0.0 {
            let _ = writeln!(s, "    RHS R{i} {r:e}");
        }
    }
    let _ = writeln!(s, "RANGES");
    for i in 0..p.m {
        if p.row_lo[i] != p.row_up[i] && p.row_lo[i].is_finite() && p.row_up[i].is_finite() {
            let _ = writeln!(s, "    RNG R{i} {:e}", p.row_up[i] - p.row_lo[i]);
        }
    }
    let _ = writeln!(s, "BOUNDS");
    for j in 0..p.n {
        let (l, u) = (p.col_lo[j], p.col_up[j]);
        if l == u {
            let _ = writeln!(s, " FX BND C{j} {l:e}");
            continue;
        }
        if l == f64::NEG_INFINITY {
            let _ = writeln!(s, " MI BND C{j}");
        } else if l != 0.0 {
            let _ = writeln!(s, " LO BND C{j} {l:e}");
        }
        if u.is_finite() {
            let _ = writeln!(s, " UP BND C{j} {u:e}");
        } else if p.is_int[j] {
            let _ = writeln!(s, " PL BND C{j}");
        }
    }
    let _ = writeln!(s, "ENDATA");
    let _ = std::fs::write(f, s);
    eprintln!("MIP: presolved problem written to {f} (objective offset {:e})", p.offset);
}

thread_local! {
    /// この求解の前処理の期限 ([`presolve_stop`])。[`solve_problem_inner`] が前処理の間だけ置く
    static PRESOLVE_DEADLINE: std::cell::Cell<Option<std::time::Instant>> = const { std::cell::Cell::new(None) };
}

/// 前処理の重い手順 (クリーク行の拡張 ([`extend_clique_rows`])・プロービング) を打ち切るか: このスレッドの止めの合図
/// ([`crate::cancel::halted`]、木の探索の従のスレッド) が立った、またはこの求解の時間の上限 (前処理の始めから数える) を過ぎた。
/// 木のヒューリスティクス役の従が出したサブ MIP (`ENOMOTO_MIP_SUBMIP_HIGHS` の `p` で親と同じ前処理) のクリーク行の拡張が、
/// 主の止めの合図の後も 4.8 秒回り、主がその従を待つ間に時間の上限を 2.7 秒過ぎていた (neos-960392、analysis/bgw_overrun_20261031.md)。
/// `ENOMOTO_MIP_NO_PRESOLVE_STOP` で使わない (切り分け用)
pub(super) fn presolve_stop() -> bool {
    (crate::cancel::halted() || crate::ticks::over() || PRESOLVE_DEADLINE.with(|d| d.get()).is_some_and(|d| std::time::Instant::now() >= d)) && env_str!("ENOMOTO_MIP_NO_PRESOLVE_STOP").is_none()
}

/// `ENOMOTO_MIP_PRESOLVE_STOP_FINE` (analysis/objtarget_20261119/ の 10 節。**2026-11-20 に既定にした: `=0`・`off` で使わない (前の形)**): プロービングで止めの合図
/// ([`presolve_stop`]) を候補ごとに見て、`ENOMOTO_MIP_PREPAR` の窓 (rayon のスレッドで先の候補を試す) の中でも見る。
/// 前の形は 16 候補ごと、窓の中では見ない。rayon のスレッドは止めの合図・前処理の期限 (どちらもスレッドごと) を受け継がないので、
/// 窓 (最大 64 x 4 候補) を試し終えるまで止まらない。nw04 の再スタートの後の FJ_P3 の前処理で、FJ のスレッドが止めの合図の後
/// 4.6〜4.9 秒回っていた
pub(super) fn presolve_stop_fine() -> bool {
    env_def!("ENOMOTO_MIP_PRESOLVE_STOP_FINE", "1").is_some()
}

/// 前処理の止めの合図と期限を、別のスレッド (rayon の窓) に渡すための写し ([`presolve_stop_fine`] のときだけ)
#[derive(Clone)]
pub(super) struct PresolveStopCtx(Option<std::sync::Arc<std::sync::atomic::AtomicBool>>, Option<std::time::Instant>);
impl PresolveStopCtx {
    pub(super) fn capture() -> Option<Self> {
        presolve_stop_fine().then(|| PresolveStopCtx(crate::cancel::inherit().flag(), PRESOLVE_DEADLINE.with(|d| d.get())))
    }
    /// 写した合図・期限をこのスレッドに置いて `f` を実行する
    pub(super) fn run<R>(this: &Option<Self>, f: impl FnOnce() -> R) -> R {
        match this {
            Some(c) => crate::cancel::with_halt(c.0.clone(), || {
                let prev = PRESOLVE_DEADLINE.with(|d| d.replace(c.1));
                let r = f();
                PRESOLVE_DEADLINE.with(|d| d.set(prev));
                r
            }),
            None => f(),
        }
    }
}

/// `deadline` を過ぎたか ([`with_presolve_deadline`] と同じく `ENOMOTO_MIP_NO_FJ_P3_DEADLINE` なら常に偽)。
pub(super) fn past_presolve_deadline(deadline: Option<std::time::Instant>) -> bool {
    env_str!("ENOMOTO_MIP_NO_FJ_P3_DEADLINE").is_none() && deadline.is_some_and(|d| std::time::Instant::now() >= d)
}

/// このスレッドの前処理の期限 ([`presolve_stop`]) を `deadline` にして `f` を実行し、元に戻す。
/// [`solve_problem_inner`] を通らずに前処理 ([`presolve_mip`]) を回す別スレッド (FJ・Local-MIP のスレッドの [`fj_p3_problem`]) が、
/// 外側の求解の期限を過ぎたらプロービングなどを打ち切るのに使う。FJ のスレッドは `std::thread::spawn` で作るので期限が付いておらず、
/// 主が根の LP を時間の上限で打ち切って `join_fj_thread` で待つ間も FJ_P3 の前処理のプロービングを回し続け、
/// cryptanalysiskb128n5obj14・16 で時間の上限を 3〜22 秒過ぎていた (analysis/overrun_crypt_20261101.md)。
/// `ENOMOTO_MIP_NO_FJ_P3_DEADLINE` で期限を置かない (切り分け用、直す前と同じ)
pub(super) fn with_presolve_deadline<R>(deadline: Option<std::time::Instant>, f: impl FnOnce() -> R) -> R {
    if env_str!("ENOMOTO_MIP_NO_FJ_P3_DEADLINE").is_some() {
        return f();
    }
    struct Restore(Option<std::time::Instant>);
    impl Drop for Restore {
        fn drop(&mut self) {
            PRESOLVE_DEADLINE.with(|d| d.set(self.0));
        }
    }
    let _restore = Restore(PRESOLVE_DEADLINE.with(|d| d.replace(deadline)));
    f()
}

/// クリーク行の拡張 (既定で使う。`ENOMOTO_MIP_NO_CLIQUE_EXTEND` で使わない。HiGHS の前処理の probing の "lifted nonzeros" に当たる):
/// 0-1 列だけで「文字の和 <= 1」の形の片側の行 (集合詰め込み行、`1 - x` の文字を含む) について、行のすべての文字と衝突する
/// (衝突グラフ = 行から作る組 + プロービングの含意 [`MipProblem::conflict_edges`]) 文字を行に加える。加えた文字どうしも
/// 衝突するものだけ足すので、行は大きなクリークのままで整数解は変わらず、LP の緩和だけが締まる。
/// 等式の行 (文字の和 = 1) では、すべての文字と衝突する文字は偽に固定できる (`ENOMOTO_MIP_CLIQUE_EXTEND_FIX`)。
/// 1 行に加える文字は `ENOMOTO_T_MIP_CLIQUE_EXTEND_MAX` 個まで (既定 1000)。拡張でまったく同じになった行は 1 本にする。
/// 試作 `ENOMOTO_MIP_CLQEXT_KNAP` (WINWHY、既定では使わない。analysis/winwhy_20261111.md) を使うか。
fn clqext_knap_on() -> bool {
    env_str!("ENOMOTO_MIP_CLQEXT_KNAP").is_some()
}

/// [`clqext_knap_on`] の本体: 0-1 列だけの行 `sum a x <= b` (>= は向きを反転、等式は両向き) で、係数が ±1 だけではないものから、
/// どの 2 つも同時に真にできない文字の組 (クリーク) を取り出し、集合詰め込み行 `sum 文字 <= 1` として返す ((行, 右辺の上限) の組。
/// 負の文字 1 - x_j は係数 -1 と右辺の -1 で表す)。負の係数の列は 1 - x_j に置き換えて全部の重み w > 0・右辺 B にし、重みの大きい順に
/// 並べて、上から k 個のうち最後の 2 つの和が B を越える最大の k を取る (重みは大きい順なので、上から k 個のどの 2 つの和も B を越える)。
/// k >= `ENOMOTO_T_MIP_CLQEXT_KNAP_MIN` (既定 3) のときだけ足す。1 つで B を越える文字 (それだけで偽に固定できる) を含む行は飛ばす
/// (前処理が扱う)。neos-950242 ではクリーク行の拡張だけで根の LP が 1 -> 4 (= 最適値) になり、HiGHS に 3 倍勝つ。拡張は係数 ±1 の
/// 行にしか当たらないので、ナップサック行に隠れたクリークにも当たるように広げる。足した行は続く拡張でさらに広げ、含まれる行は消す
fn knapsack_clique_rows(rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64], binary: &[bool], col_lo: &[f64], col_up: &[f64]) -> Vec<(Vec<(usize, f64)>, f64)> {
    let kmin = tunable!("ENOMOTO_T_MIP_CLQEXT_KNAP_MIN", 3usize, usize).max(2);
    let mut out = Vec::new();
    let mut seen: std::collections::HashSet<Vec<u32>> = std::collections::HashSet::new();
    for i in 0..rows.len() {
        let r = &rows[i];
        if r.len() < kmin || !r.iter().all(|&(j, _)| binary[j]) || r.iter().all(|&(_, a)| (a.abs() - 1.0).abs() <= 1e-12) {
            continue;
        }
        for sg in [1.0f64, -1.0] {
            let b = if sg > 0.0 { row_up[i] } else { -row_lo[i] };
            if !b.is_finite() {
                continue;
            }
            // 固定された列は右辺に移し、残りを文字と重みにする
            let mut bb = b;
            let mut lits: Vec<(f64, u32)> = Vec::new();
            for &(j, a) in r {
                let c = sg * a;
                if col_lo[j] == col_up[j] {
                    bb -= c * col_lo[j];
                } else if c > 1e-12 {
                    lits.push((c, 2 * j as u32));
                } else if c < -1e-12 {
                    // c x = c - c (1 - x): 文字 1 - x の重み -c、右辺に -c を足す
                    bb -= c;
                    lits.push((-c, 2 * j as u32 + 1));
                }
            }
            if lits.len() < kmin {
                continue;
            }
            let tol = 1e-9 * (1.0 + bb.abs());
            lits.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal));
            if lits[0].0 > bb + tol || lits[0].0 + lits[1].0 <= bb + tol {
                continue;
            }
            let mut k = 2;
            while k < lits.len() && lits[k - 1].0 + lits[k].0 > bb + tol {
                k += 1;
            }
            if k < kmin {
                continue;
            }
            let mut key: Vec<u32> = lits[..k].iter().map(|&(_, l)| l).collect();
            key.sort_unstable();
            if !seen.insert(key.clone()) {
                continue;
            }
            let nneg = key.iter().filter(|&&l| l % 2 == 1).count() as f64;
            let row: Vec<(usize, f64)> = key.iter().map(|&l| ((l / 2) as usize, if l % 2 == 0 { 1.0 } else { -1.0 })).collect();
            out.push((row, 1.0 - nneg));
        }
    }
    out
}

/// 診断 `ENOMOTO_MIP_G5CLQ_RSS` (振る舞いは変えない。analysis/vsscip_slow_20261118/g5/README.md): クリーク行の拡張の各段の RSS を標準エラーへ
fn g5clq_rss(tag: &str) {
    if env_str!("ENOMOTO_MIP_G5CLQ_RSS").is_some() {
        let rss = std::fs::read_to_string("/proc/self/status").ok().and_then(|s| s.lines().find(|l| l.starts_with("VmRSS:")).and_then(|l| l.split_whitespace().nth(1)).and_then(|v| v.parse::<u64>().ok())).unwrap_or(0);
        eprintln!("G5CLQ_RSS {tag} {} MB", rss / 1024);
    }
}

fn extend_clique_rows(prob: MipProblem, verbose: bool) -> MipProblem {
    let _pt = pretime::guard("clqext");
    g5clq_rss("start");
    let t0 = std::time::Instant::now();
    let n = prob.n;
    let binary: Vec<bool> = (0..n).map(|j| prob.is_int[j] && prob.col_lo[j] == 0.0 && prob.col_up[j] == 1.0).collect();
    // 時間の上限・止めの合図 ([`presolve_stop`]) で衝突グラフを作り切れなければ拡張しない
    let Some(g) = clique::CliqueGraph::build_with_stop(n, &prob.rows, &prob.row_lo, &prob.row_up, &binary, &prob.col_lo, &prob.col_up, &prob.conflict_edges, &presolve_stop) else {
        if verbose {
            eprintln!("MIP: clique extension: stopped while building the conflict graph (time limit or stop signal, {:.2}s); not used", t0.elapsed().as_secs_f64());
        }
        return prob;
    };
    cutmix::log_clique(&prob, &binary, g.num_edges());
    g5clq_rss("graph");
    // `ENOMOTO_MIP_CLIQUE_EXTEND_ROWS_FIRST` (試作、既定では使わない): まず行から作る衝突グラフ (プロービングの含意なし) だけで
    // 文字を足し、その後に含意を含むグラフで足せるだけ足す。含意の辺が多いと (mzzv42z は行から 205346 辺、含意から 1186997 本)、
    // 番号順の貪欲が LP に効かない文字を先に取り、行から作る辺だけの拡張 (HiGHS の前処理の clique merging に当たる) を塞ぐ。
    // mzzv42z の根の LP は既定 -21534、プロービングなし -21424 (HiGHS の前処理後の問題と同じ値)
    // 値 `fix0` は下の 0 に固定された列の扱いだけ使う (切り分け用)
    let rows_first = env_str!("ENOMOTO_MIP_CLIQUE_EXTEND_ROWS_FIRST");
    let skip_fixed0 = rows_first.is_some();
    let g_rows = (rows_first.is_some_and(|v| v != "fix0") && !prob.conflict_edges.is_empty())
        .then(|| clique::CliqueGraph::build_with(n, &prob.rows, &prob.row_lo, &prob.row_up, &binary, &prob.col_lo, &prob.col_up, &[]));
    let max_add = tunable!("ENOMOTO_T_MIP_CLIQUE_EXTEND_MAX", 1000usize, usize);
    let want_fix = env_str!("ENOMOTO_MIP_CLIQUE_EXTEND_FIX").is_some();
    let MipProblem { mut col_lo, mut col_up, cost, offset, sense_sign, is_int, mut rows, mut row_lo, mut row_up, conflict_edges, implied_bounds, .. } = prob;
    // 試作 `ENOMOTO_MIP_CLQEXT_KNAP` (WINWHY、既定では使わない。analysis/winwhy_20261111.md): ナップサック行のクリークも行にして拡張する
    let knap_rows = if clqext_knap_on() { knapsack_clique_rows(&rows, &row_lo, &row_up, &binary, &col_lo, &col_up) } else { Vec::new() };
    let n_knap = knap_rows.len();
    for (r, up) in knap_rows {
        rows.push(r);
        row_lo.push(f64::NEG_INFINITY);
        row_up.push(up);
    }
    if verbose && clqext_knap_on() {
        eprintln!("MIP: clique extension: CLQEXT_KNAP added {n_knap} clique rows from knapsack rows");
    }
    let (mut ext_rows, mut added, mut fixed) = (0usize, 0usize, 0usize);
    let mut fix_lits: Vec<u32> = Vec::new();
    // 試作 `ENOMOTO_MIP_ORDINV` の `q`: 列の鍵 (行・列の並びに依らないハッシュ)
    let okeys: Option<Vec<u64>> = crate::presolve::ordkey::ordinv('q').then(|| {
        let rr: Vec<(Vec<(usize, f64)>, f64, f64)> = rows.iter().zip(row_lo.iter().zip(&row_up)).map(|(r, (&l, &u))| (r.clone(), l, u)).collect();
        crate::presolve::ordkey::column_keys(n, &rr, &cost, &col_lo, &col_up, &is_int)
    });
    // 片側の集合詰め込み行の文字の集合 (拡張の後)。ほかの片側の集合詰め込み行に含まれる行は冗長なので消す
    // (`ENOMOTO_MIP_CLIQUE_EXTEND_KEEP_DOMINATED` で消さない)。HiGHS のクリークの統合も含まれるクリークを消す
    let mut clique_rows: Vec<(usize, Vec<u32>)> = Vec::new();
    // 途中で止めたか ([`presolve_stop`])。そこまでに足した文字はどれも衝突グラフの辺によるので正しい。支配された行の除去はしない
    let mut stopped = false;
    for i in 0..rows.len() {
        if i % 64 == 0 && presolve_stop() {
            stopped = true;
            break;
        }
        let (l, u) = (row_lo[i], row_up[i]);
        let eq = l == u;
        // sg * row <= b の向き (等式は <= の向きで見る)
        let sg = if u.is_finite() && (eq || !l.is_finite()) {
            1.0
        } else if l.is_finite() && !u.is_finite() {
            -1.0
        } else {
            continue;
        };
        let b = if sg > 0.0 { u } else { -l };
        // `ENOMOTO_MIP_CLIQUE_EXTEND_ROWS_FIRST` では 0 に固定された列 (プロービングの固定は前処理をかけ直すまで行に残る) を
        // 行にないものとして見る。既定では固定された列を含む行は集合詰め込み行と見ない
        let r: Vec<(usize, f64)> = if skip_fixed0 { rows[i].iter().copied().filter(|&(j, _)| !(col_lo[j] == 0.0 && col_up[j] == 0.0)).collect() } else { rows[i].clone() };
        if r.len() < 2 || !r.iter().all(|&(j, a)| binary[j] && ((sg * a) - 1.0).abs() <= 1e-12 || binary[j] && ((sg * a) + 1.0).abs() <= 1e-12) {
            continue;
        }
        // 文字の和 <= b + (負の係数の数)
        let nneg = r.iter().filter(|&&(_, a)| sg * a < 0.0).count() as f64;
        if ((b + nneg) - 1.0).abs() > 1e-9 {
            continue;
        }
        let lits: Vec<u32> = r.iter().map(|&(j, a)| if sg * a > 0.0 { 2 * j as u32 } else { 2 * j as u32 + 1 }).collect();
        let in_row: std::collections::HashSet<usize> = r.iter().map(|&(j, _)| j).collect();
        let mut add: Vec<u32> = Vec::new();
        // 行から作るグラフだけで足す (`ENOMOTO_MIP_CLIQUE_EXTEND_ROWS_FIRST`)。足した文字は含意を含むグラフでも互いに隣接する
        if let Some(g0) = &g_rows {
            let &l0 = lits.iter().min_by_key(|&&x| g0.neighbors(x).len()).unwrap();
            for &c in g0.neighbors(l0) {
                if add.len() >= max_add {
                    break;
                }
                let jc = (c / 2) as usize;
                if in_row.contains(&jc) || col_lo[jc] == col_up[jc] {
                    continue;
                }
                if lits.iter().all(|&x| x == l0 || g0.adjacent(x, c)) && add.iter().all(|&y| (y / 2) as usize != jc && g0.adjacent(y, c)) {
                    add.push(c);
                }
            }
        }
        // 候補: 隣接の最も少ない文字の隣接から
        // 試作 `ENOMOTO_MIP_ORDINV` の `q` (crate::presolve::ordkey): 同点の文字と候補の順を列番号でなく列の鍵の順にする
        // (候補を番号順に貪欲に足すと、並べ替えで足す文字が変わる: roll3000・ns1116954・physiciansched6-2)
        let l0 = match &okeys {
            Some(k) => *lits.iter().min_by_key(|&&x| (g.neighbors(x).len(), k[(x / 2) as usize], x % 2, x)).unwrap(),
            None => *lits.iter().min_by_key(|&&x| g.neighbors(x).len()).unwrap(),
        };
        // 候補 (行の全部の文字と隣接するもの) を、候補どうしの隣接の数の多い順 (同点は列の鍵) に並べる。番号の順の貪欲より
        // 大きなクリークを作りやすく、同じ鍵の列どうしの同点も減る。候補が多すぎるときは鍵の順だけ
        let cand_sorted: Option<Vec<u32>> = okeys.as_ref().map(|k| {
            let mut v: Vec<u32> = g
                .neighbors(l0)
                .iter()
                .copied()
                .filter(|&c| {
                    let jc = (c / 2) as usize;
                    !in_row.contains(&jc) && col_lo[jc] != col_up[jc] && lits.iter().all(|&x| x == l0 || g.adjacent(x, c))
                })
                .collect();
            if v.len() <= tunable!("ENOMOTO_T_MIP_ORDINV_CLQ_DEG_MAX", 2000usize, usize) {
                let deg: std::collections::HashMap<u32, usize> = v.iter().map(|&c| (c, v.iter().filter(|&&d| d / 2 != c / 2 && g.adjacent(c, d)).count())).collect();
                v.sort_by_key(|&x| (std::cmp::Reverse(deg[&x]), k[(x / 2) as usize], x % 2, x));
            } else {
                v.sort_by_key(|&x| (k[(x / 2) as usize], x % 2, x));
            }
            v
        });
        for &c in cand_sorted.as_deref().unwrap_or(g.neighbors(l0)) {
            if add.len() >= max_add {
                break;
            }
            let jc = (c / 2) as usize;
            if in_row.contains(&jc) || col_lo[jc] == col_up[jc] {
                continue;
            }
            if lits.iter().all(|&x| x == l0 || g.adjacent(x, c)) && add.iter().all(|&y| (y / 2) as usize != jc && g.adjacent(y, c)) {
                add.push(c);
            }
        }
        if !eq {
            let mut all: Vec<u32> = lits.iter().chain(&add).copied().collect();
            all.sort_unstable();
            clique_rows.push((i, all));
        }
        if add.is_empty() {
            continue;
        }
        if eq {
            // 等式: 行のどれかが真なので、全員と衝突する文字は偽
            if want_fix {
                fix_lits.extend(add.iter().copied());
            }
            continue;
        }
        let mut b2 = b;
        for &c in &add {
            let jc = (c / 2) as usize;
            if c % 2 == 0 {
                rows[i].push((jc, sg));
            } else {
                rows[i].push((jc, -sg));
                b2 -= 1.0;
            }
        }
        rows[i].sort_unstable_by_key(|e| e.0);
        if sg > 0.0 {
            row_up[i] = b2;
        } else {
            row_lo[i] = -b2;
        }
        ext_rows += 1;
        added += add.len();
    }
    g5clq_rss("loop");
    for c in fix_lits {
        let j = (c / 2) as usize;
        if col_lo[j] < col_up[j] {
            // 文字 x_j (c 偶数) が偽なら x_j = 0、文字 1 - x_j が偽なら x_j = 1
            if c % 2 == 0 {
                col_up[j] = 0.0;
            } else {
                col_lo[j] = 1.0;
            }
            fixed += 1;
        }
    }
    // 拡張でまったく同じになった行を 1 本にする
    let mut seen: std::collections::HashMap<(Vec<(usize, u64)>, u64, u64), ()> = std::collections::HashMap::new();
    let mut keep = vec![true; rows.len()];
    let mut dup = 0usize;
    let mut dominated = 0usize;
    if !stopped && env_str!("ENOMOTO_MIP_CLIQUE_EXTEND_KEEP_DOMINATED").is_none() {
        // 文字 -> それを含むクリーク行 (clique_rows の添字)
        let mut by_lit: std::collections::HashMap<u32, Vec<usize>> = std::collections::HashMap::new();
        for (c, (_, ls)) in clique_rows.iter().enumerate() {
            for &l in ls {
                by_lit.entry(l).or_default().push(c);
            }
        }
        let mut order: Vec<usize> = (0..clique_rows.len()).collect();
        order.sort_by_key(|&c| clique_rows[c].1.len());
        let mut gone = vec![false; clique_rows.len()];
        for &c in &order {
            let (i, ls) = &clique_rows[c];
            // 含む行の候補: 文字のうち含む行の最も少ないものから
            let Some(l0) = ls.iter().min_by_key(|l| by_lit.get(l).map_or(0, |v| v.len())) else { continue };
            let found = by_lit[l0].iter().any(|&d| {
                d != c && !gone[d] && {
                    let ld = &clique_rows[d].1;
                    (ld.len() > ls.len() || (ld.len() == ls.len() && d < c)) && ls.iter().all(|x| ld.binary_search(x).is_ok())
                }
            });
            if found {
                gone[c] = true;
                keep[*i] = false;
                dominated += 1;
            }
        }
    }
    for i in 0..rows.len() {
        if !keep[i] {
            continue;
        }
        let key = (rows[i].iter().map(|&(j, a)| (j, a.to_bits())).collect::<Vec<_>>(), row_lo[i].to_bits(), row_up[i].to_bits());
        if seen.insert(key, ()).is_some() {
            keep[i] = false;
            dup += 1;
        }
    }
    g5clq_rss("dominated");
    if dup + dominated > 0 {
        let mut k = 0;
        rows.retain(|_| {
            k += 1;
            keep[k - 1]
        });
        let mut k = 0;
        row_lo.retain(|_| {
            k += 1;
            keep[k - 1]
        });
        let mut k = 0;
        row_up.retain(|_| {
            k += 1;
            keep[k - 1]
        });
    }
    if verbose {
        eprintln!("MIP: clique extension: {ext_rows} rows extended, {added} nonzeros added, {fixed} columns fixed, {dup} duplicate and {dominated} dominated rows removed ({} edges, {:.2}s){}", g.num_edges(), t0.elapsed().as_secs_f64(), if stopped { " (stopped by the time limit or the stop signal)" } else { "" });
    }
    let mut q = MipProblem::from_rows(col_lo, col_up, cost, offset, sense_sign, is_int, rows, row_lo, row_up);
    q.conflict_edges = conflict_edges;
    q.implied_bounds = implied_bounds;
    q
}

/// 試作 `ENOMOTO_MIP_DUALEQ` (既定では使わない、analysis/presolve2_20261111.md): 双対の議論による行の等式化。列 `j` を目的の
/// 悪くならない向き (費用 `c_j <= 0` なら増やす、`c_j >= 0` なら減らす) に動かしたとき破れうる行 (錠) がちょうど 1 本 `r` なら、
/// どの最適解からも `x_j` をその向きに `r` が等号になるまで動かせば、目的は悪くならず、ほかの行は緩む向きにしか動かない。
/// ただし (1) `r` が等号になる前に `x_j` の境界に当たらない (`r` から導かれる `x_j` の境界が `x_j` の境界より内側)、
/// (2) `x_j` が整数列なら、`r` のほかの列がすべて整数列で、係数と右辺が `a_rj` の整数倍 (動かす幅が整数になる) のときだけ。
/// このとき `r` を等式にしても最適解が 1 つは残る。1 回の呼び出しで複数の組 (j, r) を選ぶときは、選んだ列はほかの選んだ行に
/// 現れないようにする (動かす順によらず等号が保たれる)。HiGHS はこの形で neos859080 の `y_a + y_b <= 1` (y_b は費用 0 で、
/// ほかには `x_k <= 10 y_b` にしか出ない) を等式にして `y_b = 1 - y_a` と代入し、0-1 列 40 本を消す (等式は既定の 2 項等式の
/// 消去が代入する)。等式にした行の数を返す。最適解の集合を狭めるだけ (実行可能な点は変えない) なので後処理は要らない
#[allow(clippy::too_many_arguments)]
fn dual_row_equalities(rows: &[Vec<(usize, f64)>], row_lo: &mut [f64], row_up: &mut [f64], cols: &[Vec<(usize, f64)>], cost: &[f64], is_int: &[bool], col_lo: &[f64], col_up: &[f64]) -> usize {
    let m = rows.len();
    let n = cols.len().min(cost.len());
    let integral = |v: f64| (v - v.round()).abs() <= 1e-9 * v.abs().max(1.0);
    // 等式にした行と、選んだ列が現れる行 (後で選ぶ行はこれらを含まない)
    let mut used_row = vec![false; m];
    let mut blocked_row = vec![false; m];
    let mut count = 0usize;
    for j in 0..n {
        // 1 行にしか出ない列は列シングルトンの縮約 (不等式のまま扱う方が強い) に任せる
        if col_lo[j] >= col_up[j] || cols[j].len() < 2 {
            continue;
        }
        for dir in [1.0f64, -1.0] {
            if dir * cost[j] > 0.0 {
                continue;
            }
            // 錠の行: x_j を dir の向きに動かすと破れうる側を持つ行
            let mut lock: Option<(usize, f64)> = None;
            let mut cnt = 0usize;
            for &(i, a) in &cols[j] {
                if i >= m || a == 0.0 {
                    continue;
                }
                let s = a * dir;
                if (s > 0.0 && row_up[i].is_finite()) || (s < 0.0 && row_lo[i].is_finite()) {
                    cnt += 1;
                    lock = Some((i, a));
                }
            }
            if cnt != 1 {
                continue;
            }
            let (r, a) = lock.unwrap();
            if row_lo[r] == row_up[r] || used_row[r] || blocked_row[r] || cols[j].iter().any(|&(i, _)| i < m && used_row[i]) {
                continue;
            }
            let upper = a * dir > 0.0;
            let rhs = if upper { row_up[r] } else { row_lo[r] };
            // 等式にすると r のほかの列 k に逆の向きの錠が 1 本増える。その向きに錠がなく目的も悪くならない列 (双対の固定で
            // 境界に固定できる列) があれば選ばない (gt2 では `Σ x <= 6` の費用 0 の列 8 本の固定を失っていた)
            let breaks_dualfix = rows[r].iter().any(|&(k, b)| {
                if k == j || k >= n || b == 0.0 {
                    return false;
                }
                let dk = if upper { -b.signum() } else { b.signum() };
                dk * cost[k] <= 0.0
                    && !cols[k].iter().any(|&(i, c)| i < m && ((c * dk > 0.0 && row_up[i].is_finite()) || (c * dk < 0.0 && row_lo[i].is_finite())))
            });
            if breaks_dualfix {
                continue;
            }
            // r のほかの列の活動の範囲と、整数性の条件
            let (mut smin, mut smax) = (0.0f64, 0.0f64);
            let mut int_ok = true;
            for &(k, b) in &rows[r] {
                if k == j {
                    continue;
                }
                let (lo, up) = (col_lo[k], col_up[k]);
                if b > 0.0 {
                    smin += b * lo;
                    smax += b * up;
                } else {
                    smin += b * up;
                    smax += b * lo;
                }
                if is_int[j] && (!is_int[k] || !integral(b / a)) {
                    int_ok = false;
                }
            }
            if is_int[j] && (!int_ok || !integral(rhs / a)) {
                continue;
            }
            // 等号にしたときの x_j = (rhs - S) / a の、dir の向きの端が x_j の境界を越えない
            let bound = if dir > 0.0 { col_up[j] } else { col_lo[j] };
            if bound.is_finite() {
                let (x1, x2) = ((rhs - smin) / a, (rhs - smax) / a);
                if !x1.is_finite() || !x2.is_finite() {
                    continue;
                }
                let far = if dir > 0.0 { x1.max(x2) } else { x1.min(x2) };
                if dir * (far - bound) > 1e-9 * (1.0 + bound.abs()) {
                    continue;
                }
            }
            if upper {
                row_lo[r] = row_up[r];
            } else {
                row_up[r] = row_lo[r];
            }
            used_row[r] = true;
            for &(i, _) in &cols[j] {
                if i < m {
                    blocked_row[i] = true;
                }
            }
            count += 1;
            break;
        }
    }
    count
}

/// 試作 `ENOMOTO_MIP_TREE2_FORCEQ` (既定では使わない、analysis/tree_close2_20261104.md): 等式行 `Σ_{k≠y} a_k x_k + a_y y = b` で、
/// `y` が 0-1 列 1 本、残りの列の活動の範囲 `[L, U]` について `y = 0` と `y = 1` の右辺 (`b` と `b - a_y`) が `L` と `U` の
/// 一方ずつに一致するもの (強制行が `y` の値ごとに決まる) から、残りの列ごとに `x_k = v0_k + (v1_k - v0_k) y` の 2 項等式を作る
/// (`v0_k`・`v1_k` は `y = 0`・`y = 1` で強制される境界)。整数解では元の行と同値で、LP の緩和だけが締まる。足した等式は
/// 既定の前処理 (2 項等式・Aggregator) が連続列の代入に使う。HiGHS は neos-1445765 でこの形の行 (`Σ x = K y`、x は [0,1] の
/// 連続列が K 本) から連続列 18467 本を消し、根の LP が -24699 -> -18811 になる。連続列だけを対象にし、行の長さは
/// `ENOMOTO_T_MIP_TREE2_FORCEQ_MAXLEN` (既定 1000) まで
fn tree2_forceq_rows(rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64], col_lo: &[f64], col_up: &[f64], is_int: &[bool]) -> Vec<(Vec<(usize, f64)>, f64)> {
    let maxlen = tunable!("ENOMOTO_T_MIP_TREE2_FORCEQ_MAXLEN", 1000usize, usize);
    let mut out = Vec::new();
    let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    for (i, r) in rows.iter().enumerate() {
        if row_lo[i] != row_up[i] || !row_lo[i].is_finite() || r.len() < 2 || r.len() > maxlen {
            continue;
        }
        let b = row_lo[i];
        // 0-1 列がちょうど 1 本、残りは有限の境界を持つ連続列
        let mut y: Option<(usize, f64)> = None;
        let mut ok = true;
        for &(j, a) in r {
            if a == 0.0 {
                continue;
            }
            if is_int[j] && col_lo[j] == 0.0 && col_up[j] == 1.0 {
                if y.is_some() {
                    ok = false;
                    break;
                }
                y = Some((j, a));
            } else if is_int[j] || !col_lo[j].is_finite() || !col_up[j].is_finite() || col_lo[j] == col_up[j] {
                ok = false;
                break;
            }
        }
        let Some((yj, ay)) = y else { continue };
        if !ok {
            continue;
        }
        let (mut lo, mut hi, mut mag) = (0.0f64, 0.0f64, b.abs() + ay.abs());
        for &(j, a) in r {
            if j == yj || a == 0.0 {
                continue;
            }
            let (l, u) = if a > 0.0 { (a * col_lo[j], a * col_up[j]) } else { (a * col_up[j], a * col_lo[j]) };
            lo += l;
            hi += u;
            mag += l.abs() + u.abs();
        }
        // 一致の許容誤差は絶対値 1e-9 (和の丸めの分だけ `mag` に比例して広げる)。以前は `1e-9 * mag` で、unitcal_7 (DUALEQ が
        // `x_a + x_b - 1000 y <= 0` を等式にした行、x_b の上限 1.002e-6) で上限との差 1.002e-6 を一致と見なし、y = 1 で
        // `x_b = 1.002e-6` と決めつけて実行可能な点 (x_b = 0, x_a = 1000) を切り、根の LP が実行不能になった
        // (analysis/review2_20261113.md の誤答の節)
        let tol = 1e-9f64.max(8.0 * f64::EPSILON * mag);
        let (r0, r1) = (b, b - ay);
        // `y = 0` で下側に強制されるか (`r0 = L`, `r1 = U`)、上側か (`r0 = U`, `r1 = L`)
        let low_at_0 = if (r0 - lo).abs() <= tol && (r1 - hi).abs() <= tol {
            true
        } else if (r0 - hi).abs() <= tol && (r1 - lo).abs() <= tol {
            false
        } else {
            continue;
        };
        for &(j, a) in r {
            if j == yj || a == 0.0 || !seen.insert((j, yj)) {
                continue;
            }
            // 行の活動が下側 (L) に強制されるとき x_j は a > 0 なら下限、a < 0 なら上限
            let at_low = if a > 0.0 { col_lo[j] } else { col_up[j] };
            let at_high = if a > 0.0 { col_up[j] } else { col_lo[j] };
            let (v0, v1) = if low_at_0 { (at_low, at_high) } else { (at_high, at_low) };
            // x_j - (v1 - v0) y = v0
            let mut row = vec![(j, 1.0), (yj, -(v1 - v0))];
            row.sort_unstable_by_key(|&(k, _)| k);
            out.push((row, v0));
        }
    }
    out
}

/// 前処理の結果。
enum Presolved {
    Infeasible,
    Reduced { prob: MipProblem, postsolve: Vec<crate::presolve::PostsolveStep>, scaling: crate::presolve::scaling::Scaling },
}

thread_local! {
    /// 試作 `ENOMOTO_MIP_HNFPRE_LAT`: 書き直した問題を解いている間 (入れ子で書き直さない)
    static HNF_LAT_ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 試作 `ENOMOTO_MIP_HNFPRE`: 前処理で合同式が見つかり、解く直前に表を作ることになっているか
    static HNF_PENDING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 試作 `ENOMOTO_MIP_HNFPRE`: 解く直前の問題 `prob` で整数列だけの等式を解析し直し、合同式の表を登録する (分枝・伝播の丸めに使う)。
fn hnf_register_final(prob: &MipProblem, d: &[f64]) {
    let int_ok: Vec<bool> = (0..prob.n).map(|j| d.get(j).is_none_or(|&d| d == 1.0)).collect();
    let res = hnf::analyze(prob, &int_ok);
    let t = if res.stats.infeasible { None } else { hnf::make_table(prob, &res.info) };
    if env_str!("ENOMOTO_MIP_HNFPRE_QUIET").is_none() {
        eprintln!("HNFPRE: final table: {} congruences ({:.3}s)", t.as_ref().map_or(0, |t| t.m.iter().filter(|&&m| m > 1).count()), res.stats.secs);
    }
    if let Some(t) = t {
        hnf::register(t);
    }
}

/// 試作 `ENOMOTO_MIP_HNFPRE`: 前処理の後の問題の整数列だけの等式を Hermite 標準形で解析し ([`hnf::analyze`])、実行不能なら
/// `Infeasible`、固定・合同式で上下限が詰まれば詰めて前処理をかけ直す (尺度が掛かっていなければ。最大 3 回)。`Full` なら最後の
/// 問題の合同式を登録する (分枝・伝播の丸めに使う、[`hnf::table_for`])。
fn hnf_presolve(mut prob: MipProblem, mut postsolve: Vec<crate::presolve::PostsolveStep>, scaling: crate::presolve::scaling::Scaling, pre_submip: bool) -> Option<Presolved> {
    // `ENOMOTO_MIP_HNFSUBST` だけなら `bounds` と同じ (実行不能・固定・上下限の丸め) をしてから置き換える
    let mode = hnf::mode().or(hnfsubst::on().then_some(hnf::Mode::Bounds))?;
    let quiet = env_str!("ENOMOTO_MIP_HNFPRE_QUIET").is_some();
    for round in 0..4 {
        let int_ok: Vec<bool> = (0..prob.n).map(|j| scaling.d.get(j).is_none_or(|&d| d == 1.0)).collect();
        let res = hnf::analyze(&prob, &int_ok);
        let s = &res.stats;
        if !quiet {
            eprintln!(
                "HNFPRE: round {round} eq {} int_eq {} rat_skip {} comps {} done {} gave_up {} cols {} rank {} fixes {} congs {} (gen {}, tighten {}, max_mod {}) infeasible {} ({:.3}s) gave_up_why {}/{}/{}",
                s.eq_rows, s.int_eq_rows, s.rat_skipped, s.comps, s.comps_done, s.comps_gave_up, s.cols, s.rank, s.fixes, s.congs, s.congs_gen, s.congs_tighten, s.max_mod, s.infeasible, s.secs, s.gave_overflow, s.gave_work, s.gave_size
            );
        }
        if mode == hnf::Mode::Count {
            return Some(Presolved::Reduced { prob, postsolve, scaling });
        }
        if s.infeasible {
            return Some(Presolved::Infeasible);
        }
        let (mut lo, mut up) = (prob.col_lo.clone(), prob.col_up.clone());
        let k = match hnf::apply_bounds(&mut lo, &mut up, &res.info) {
            Err(()) => {
                if !quiet {
                    eprintln!("HNFPRE: bounds and the lattice conflict: infeasible");
                }
                return Some(Presolved::Infeasible);
            }
            Ok(k) => k,
        };
        if !quiet {
            eprintln!("HNFPRE: round {round} tightened {k} columns");
        }
        // 試作 `ENOMOTO_T_MIP_HNFPRE_REPRE_MIN` (既定 1 = 今まで通り、analysis/slow2_20261117): 締めた列がこの本数より少なければ、
        // 前処理を (プロービングごと) かけ直さず、締めた境界だけ置いて返す。mzzv42z の再スタートでは 1 列締めるたびのかけ直しが 2 回で 11.6 秒
        if k == 0 || round == 3 || k < tunable!("ENOMOTO_T_MIP_HNFPRE_REPRE_MIN", 1usize, usize) {
            if k > 0 {
                prob.col_lo = lo;
                prob.col_up = up;
            }
            // 合同式があれば、解く直前の最終の問題 (クリーク行・対称性の行の追加、前処理のかけ直し、双子の併合の後) で解析を
            // やり直して表を作る ([`hnf_register_final`])。ここで作った表を使うと、その後の変換で列の意味が変わったときに誤る
            if mode == hnf::Mode::Full && res.info.iter().any(|&(j, _, g)| g > 1 && prob.col_lo[j] < prob.col_up[j]) {
                HNF_PENDING.with(|f| f.set(true));
            }
            return Some(Presolved::Reduced { prob, postsolve, scaling });
        }
        prob.col_lo = lo;
        prob.col_up = up;
        if !scaling.d.iter().all(|&d| d == 1.0) {
            continue;
        }
        match presolve_mip(&prob, false, pre_submip, None) {
            Some(Presolved::Infeasible) => return Some(Presolved::Infeasible),
            Some(Presolved::Reduced { prob: p2, postsolve: post2, scaling: sc2 }) if sc2.d.iter().all(|&d| d == 1.0) && p2.n == prob.n => {
                postsolve.extend(post2);
                prob = p2;
            }
            _ => {}
        }
    }
    Some(Presolved::Reduced { prob, postsolve, scaling })
}

/// 試作 `ENOMOTO_MIP_HNFSUBST`: [`hnf_presolve`] の後の問題を解析し直し、合同式 `x_j ≡ r_j (mod g_j)` (g_j > 1) の出た一般整数列を
/// `x_j = r_j + g_j y_j` に置き換える ([`hnfsubst::substitute`])。後処理の手順 (自己参照の代入) を最後に足す。
/// 前処理の中で置いた前処理後の空間の値 (早めの FJ の解) は y に写す。根の基底は列の状態をそのまま使う (下の注)。
fn hnf_subst(mut prob: MipProblem, mut postsolve: Vec<crate::presolve::PostsolveStep>, scaling: crate::presolve::scaling::Scaling, pre_submip: bool) -> (Option<Presolved>, Vec<hnfsubst::Subst>) {
    let int_ok: Vec<bool> = (0..prob.n).map(|j| scaling.d.get(j).is_none_or(|&d| d == 1.0)).collect();
    let res = hnf::analyze(&prob, &int_ok);
    let fp0 = rows_fingerprint(&prob.rows);
    let map = if res.stats.infeasible { Vec::new() } else { hnfsubst::substitute(&mut prob, &res.info, &int_ok) };
    if env_str!("ENOMOTO_MIP_HNFSUBST_QUIET").is_none() {
        let gens = res.info.iter().filter(|&&(j, _, g)| g > 1 && prob.col_lo[j] < prob.col_up[j]).count();
        eprintln!("HNFSUBST: substituted {} of {gens} congruence columns (max modulus {}) ({:.3}s)", map.len(), map.iter().map(|s| s.g).fold(0.0, f64::max), res.stats.secs);
    }
    if map.is_empty() {
        return (Some(Presolved::Reduced { prob, postsolve, scaling }), map);
    }
    postsolve.extend(map.iter().map(hnfsubst::postsolve_step));
    FEAS_FJ_SOL.with(|s| {
        if let Some(x) = s.borrow_mut().as_mut() {
            hnfsubst::forward(&map, x);
        }
    });
    // 前処理の LP による固定の基底 (列の状態) はそのまま使える (有限の上下限は合同式に合うよう詰めてあるので、x の上下限の
    // 状態は y の上下限の状態。基本列は基本列)。行は係数・右辺が変わるだけで並びは同じなので、行の指紋が置き換えの前の行と
    // 合っていたなら置き換えの後の行の指紋にする (合わなくなると行の状態が使われず、根の LP が遠い基底から解き直す: snp-02-004-104
    // で根の LP が 6 秒で終わらなかった)。DSE の重み・行の中身の写しは捨てる
    let fp1 = rows_fingerprint(&prob.rows);
    solver::ROOT_BASIS_ROWS.with(|s| {
        let mut s = s.borrow_mut();
        if *s == Some(fp0) {
            *s = Some(fp1);
        }
    });
    solver::ROOT_DSE.with(|s| *s.borrow_mut() = None);
    solver::ROOT_BASIS_SAVED.with(|s| *s.borrow_mut() = None);
    solver::PROBE_BASIS_INFO.with(|s| *s.borrow_mut() = None);
    solver::RESTART_BASIS.with(|s| *s.borrow_mut() = None);
    solver::RESTART_X.with(|s| *s.borrow_mut() = None);
    // `ENOMOTO_MIP_HNFSUBST=re` (改良 11-08、試作): 置き換えた問題に前処理をかけ直す (割った行の丸めから上下限の伝播・係数の締め付け)。
    // 尺度が掛からず列の数が同じときだけ使い、後処理の手順は置き換えの手順の後に並べる (逆順に戻すので、かけ直しの手順が先に戻る)
    if env_str!("ENOMOTO_MIP_HNFSUBST") == Some("re") && scaling.d.iter().all(|&d| d == 1.0) {
        let (m0, free0) = (prob.m, (0..prob.n).filter(|&j| prob.col_lo[j] < prob.col_up[j]).count());
        match presolve_mip(&prob, false, pre_submip, None) {
            Some(Presolved::Infeasible) => return (Some(Presolved::Infeasible), map),
            Some(Presolved::Reduced { prob: p2, postsolve: post2, scaling: sc2 }) if sc2.d.iter().all(|&d| d == 1.0) && p2.n == prob.n => {
                if env_str!("ENOMOTO_MIP_HNFSUBST_QUIET").is_none() {
                    let free1 = (0..p2.n).filter(|&j| p2.col_lo[j] < p2.col_up[j]).count();
                    eprintln!("HNFSUBST: re-presolved: rows {m0} -> {}, free columns {free0} -> {free1}", p2.m);
                }
                postsolve.extend(post2);
                prob = p2;
                // 早めの FJ の解はかけ直した問題で実行不能なら捨てる (かけ直しは列を固定・代入するだけ)
                FEAS_FJ_SOL.with(|s| {
                    let bad = s.borrow().as_ref().is_some_and(|x| x.len() != prob.n || !prob.is_feasible(x, 1e-6));
                    if bad {
                        *s.borrow_mut() = None;
                    }
                });
            }
            _ => {}
        }
    }
    (Some(Presolved::Reduced { prob, postsolve, scaling }), map)
}

/// 整数を考慮した前処理をかけた問題を作る。前処理が何も減らさなければ `None` (元の問題で解く)。
/// `ENOMOTO_MIP_CLQPRB` の `h` (試作): 既定の形 (含意を使わない) のプロビングで前処理をやり直した問題で locks (lock の多い整数列から
/// lock の少ない側に固定して伝播し、固定率 `ENOMOTO_T_MIP_CLQPRB_H_RATE` (0.65) で止める。[`heur_scip`] の locks と同じ形) の
/// sub-MIP を解き、解を元の問題の空間に戻して返す (元の問題で実行可能なときだけ)。
/// neos-3216931-puriri: 含意を使う形の前処理で締まった問題では locks の固定が 3 割台でも実行不能の組に入り、解が出なくなる。
/// 既定の形の問題では locks の sub-MIP が 81380 前後を出す
fn clqprb_shadow_locks(p: &MipProblem, params: MipParams, pre_submip: bool, lp_probe_secs: Option<f64>) -> Option<Vec<f64>> {
    let t0 = std::time::Instant::now();
    // 前処理が触る印を退避して戻す (外側の前処理の結果は変えない)
    let saved = (PROBE_EQUIV.with(|e| e.borrow().clone()), PROBE_EQUIV_FORCE.with(|f| f.get()), EQUIV2_USED.with(|f| f.get()), CLQPRB_STICKY.with(|f| f.get()), CLQPRB_REVERTED.with(|f| f.get()));
    CLQPRB_SUPPRESS.with(|f| f.set(true));
    let pd = presolve_mip(p, false, pre_submip, lp_probe_secs);
    CLQPRB_SUPPRESS.with(|f| f.set(false));
    PROBE_EQUIV.with(|e| *e.borrow_mut() = saved.0);
    PROBE_EQUIV_FORCE.with(|f| f.set(saved.1));
    EQUIV2_USED.with(|f| f.set(saved.2));
    CLQPRB_STICKY.with(|f| f.set(saved.3));
    CLQPRB_REVERTED.with(|f| f.set(saved.4));
    let t_pre = t0.elapsed().as_secs_f64();
    let Some(Presolved::Reduced { prob: q, postsolve, scaling }) = pd else {
        if params.verbose {
            eprintln!("MIP: CLQPRB=h: the default presolve gave no reduced problem ({t_pre:.2}s)");
        }
        return None;
    };
    let locks = heuristics::compute_locks(&q);
    let mut dom = domain::Domain::new(&q);
    if !dom.propagate(&q) {
        return None;
    }
    let ints: Vec<usize> = (0..q.n).filter(|&j| q.is_int[j] && dom.lo[j] < dom.up[j]).collect();
    if ints.is_empty() {
        return None;
    }
    let mut order: Vec<(u32, usize)> = ints.iter().map(|&j| (locks[j].0 + locks[j].1, j)).collect();
    order.sort_by(|a, b| b.0.cmp(&a.0));
    let stop_rate = tunable!("ENOMOTO_T_MIP_CLQPRB_H_RATE", 0.65, f64);
    let nint = ints.len() as f64;
    let rate = |d: &domain::Domain| ints.iter().filter(|&&j| d.lo[j] == d.up[j]).count() as f64 / nint;
    let work_cap = 20 * q.rows.iter().map(|r| r.len() as u64).sum::<u64>() + 1_000_000;
    let work0 = dom.debug_work();
    for (k, &(_, j)) in order.iter().enumerate() {
        if k % 64 == 0 && dom.debug_work() - work0 > work_cap {
            break;
        }
        let (dl, ul) = locks[j];
        let (lo, up) = (dom.lo[j], dom.up[j]);
        if lo == up {
            continue;
        }
        let v = if dl <= ul { lo } else { up };
        let v = if v.is_finite() { v } else if lo.is_finite() { lo } else { up };
        if !v.is_finite() {
            continue;
        }
        let before = dom.stack_len();
        dom.tighten_lower(&q, j, v);
        dom.tighten_upper(&q, j, v);
        if !dom.propagate(&q) {
            dom.backtrack_to(&q, before);
            continue;
        }
        if k % 16 == 0 && rate(&dom) >= stop_rate {
            break;
        }
    }
    let fr = rate(&dom);
    if fr < 0.3 {
        if params.verbose {
            eprintln!("MIP: CLQPRB=h: fixing rate {fr:.2} on the default problem; no sub-MIP");
        }
        return None;
    }
    let mut sub = q.clone();
    sub.col_lo = dom.lo.clone();
    sub.col_up = dom.up.clone();
    for j in 0..sub.n {
        if sub.col_lo[j] > sub.col_up[j] {
            let v = if sub.is_int[j] { sub.col_lo[j].round() } else { 0.5 * (sub.col_lo[j] + sub.col_up[j]) };
            sub.col_lo[j] = v;
            sub.col_up[j] = v;
        }
    }
    let sp = MipParams {
        rel_gap: params.rel_gap.max(1e-4),
        abs_gap: params.abs_gap.max(1e-6),
        ..MipParams::submip_base(
            tunable!("ENOMOTO_T_MIP_CLQPRB_H_SECS", 30.0, f64).min(0.1 * params.time_limit),
            tunable!("ENOMOTO_T_MIP_CLQPRB_H_NODES", 500u64, u64),
            params.seed,
        )
    };
    let r = solve_problem(&sub, sp, true);
    let found = r.x.filter(|x| x.len() == q.n).map(|mut x| {
        for step in postsolve.iter().rev() {
            step.apply(&mut x);
        }
        crate::presolve::scaling::unscale_x(&scaling, &x)
    });
    let ok = found.as_ref().is_some_and(|x| x.len() == p.n && p.is_feasible(x, 1e-6));
    if params.verbose {
        eprintln!(
            "MIP: CLQPRB=h: locks on the default problem: fixing rate {fr:.2}, sub-MIP {:?}, {} nodes, objective {:?}, feasible in the original problem {ok} (presolve {t_pre:.2}s, total {:.2}s)",
            r.status,
            r.nodes,
            found.as_ref().filter(|_| ok).map(|x| p.objective(x)),
            t0.elapsed().as_secs_f64()
        );
    }
    found.filter(|_| ok)
}

/// 試作 `ENOMOTO_MIP_ENUM_ADAPT` の `t` (analysis/nrashare_20261126 7・8 節): 前処理を回すスレッドに ENUM_ADAPT の時計 (`ENUM_T0`、主のスレッドの
/// 求解の始めに置く) がなければ、この前処理の始めを時計にし、見張りを捨てるときに元に戻す ([`presolve_mip`] の入口で作る。入れ子の呼び出しは時計が
/// あるので何もしない)。時計がないと経過は無限大で「c = 0 で常に続ける」になり、主でないスレッドの前処理 (FJ のスレッドの FJ_P3、PRECHAIN・TPRE・
/// 並列のサブ MIP など) で列挙とプロービングの繰り返し・かけ直しが止まらなかった (neos-631710 の 4 スレッドの FJ_P3 で 10.1 秒)
struct EnumClockGuard(Option<Option<(usize, f64, f64)>>);

impl EnumClockGuard {
    fn new() -> Self {
        let on = enum_adapt_on() && env_str!("ENOMOTO_MIP_ENUM_ADAPT").is_some_and(|v| v.contains('t')) && ENUM_T0.with(|c| c.get()).is_none();
        if !on {
            return EnumClockGuard(None);
        }
        let prev = ENUM_STEP_LAST.with(|c| c.replace(None));
        ENUM_T0.with(|c| c.set(Some(std::time::Instant::now())));
        EnumClockGuard(Some(prev))
    }
}

impl Drop for EnumClockGuard {
    fn drop(&mut self) {
        if let Some(prev) = self.0 {
            ENUM_T0.with(|c| c.set(None));
            ENUM_STEP_LAST.with(|c| c.set(prev));
        }
    }
}

fn presolve_mip(p: &MipProblem, verbose: bool, submip: bool, lp_probe_secs: Option<f64>) -> Option<Presolved> {
    let _enum_clock = EnumClockGuard::new();
    // 試作 `ENOMOTO_MIP_INTSING` の `h` (既定はオフ、[`crate::presolve::colsingleton::INTSING_HOLD`]): 最上段の呼び出しの間は INTSING を保留し、
    // 支配される 0-1 列の固定の後のかけ直しで消す。入れ子の呼び出し (かけ直し) は状態をそのまま受け継ぐ
    use crate::presolve::colsingleton::{intsing_domfirst, intsing_order, INTSING_HELD, INTSING_HOLD};
    // `ENOMOTO_MIP_INTSING` の `o` (2026-11-19 から既定の `1go` に含む、analysis/slowm_20261119 §13.4): 最上段の呼び出しで、支配される 0-1 列の固定
    // ([`dominated_binary_fixings`]) を前処理の前の元の問題に先にかける (固定が自由な列の `ENOMOTO_T_MIP_DOMCOL_MIN_FRAC` に満たなければ捨てる、
    // 後の段と同じ規則)。INTSING が列の費用を行の他の列へ移して費用が負の列を作る前に固定を済ませる (air03: 1g では固定 2297 → 160 本)。
    // 固定を捨てた問題は、`o` なしとまったく同じ前処理になる
    // INTSING が働かない問題を変えないよう、元の問題に「費用を移すと費用 ≥ 0 の列の費用が負になる」整数列シングルトンがあるときだけ
    let cost_negating_singleton = |p: &MipProblem| {
        let isint = |v: f64| (v - v.round()).abs() <= 1e-12 * (1.0 + v.abs());
        (0..p.n).any(|j| {
            if !p.is_int[j] || p.cols[j].len() != 1 || p.cost[j] == 0.0 || p.col_lo[j] >= p.col_up[j] {
                return false;
            }
            let (i, aj) = p.cols[j][0];
            if p.row_lo[i] != p.row_up[i] || !isint(p.row_lo[i]) || (aj.abs() - 1.0).abs() > 1e-12 {
                return false;
            }
            let row = &p.rows[i];
            row.iter().all(|&(k, v)| k == j || (p.is_int[k] && isint(v)))
                && row.iter().any(|&(k, v)| k != j && p.cost[k] >= 0.0 && p.cost[k] - p.cost[j] * v / aj < -1e-9 * (1.0 + p.cost[k].abs()))
        })
    };
    if !submip && INTSING_HOLD.with(|h| h.get()) == 0 && !DOMCOL_REPRESOLVING.with(|f| f.get()) && env_def!("ENOMOTO_MIP_DOMCOL", "1").is_some() && intsing_domfirst() && cost_negating_singleton(p) {
        let t0 = std::time::Instant::now();
        let fix = dominated_binary_fixings(p);
        let nfree = (0..p.n).filter(|&j| p.col_lo[j] < p.col_up[j]).count();
        let min_frac = tunable!("ENOMOTO_T_MIP_DOMCOL_MIN_FRAC", 0.1, f64);
        let keep = !fix.is_empty() && (fix.len() as f64) >= min_frac * nfree as f64;
        if verbose {
            eprintln!("MIP: presolve: {} dominated binaries before the presolve, {} (of {nfree} free columns, {:.2}s, ENOMOTO_MIP_INTSING o)", fix.len(), if keep { "fixed" } else { "dropped" }, t0.elapsed().as_secs_f64());
        }
        if keep {
            let mut q = p.clone();
            for &(j, v) in &fix {
                q.col_lo[j] = v;
                q.col_up[j] = v;
            }
            INTSING_HOLD.with(|h| h.set(3));
            let r = presolve_mip_depth(&q, verbose, submip, lp_probe_secs, 0);
            INTSING_HOLD.with(|h| h.set(0));
            return r;
        }
    }
    if !submip && INTSING_HOLD.with(|h| h.get()) == 0 && !DOMCOL_REPRESOLVING.with(|f| f.get()) && env_def!("ENOMOTO_MIP_DOMCOL", "1").is_some() && intsing_order() {
        INTSING_HOLD.with(|h| h.set(1));
        INTSING_HELD.with(|h| h.set(0));
        let r = presolve_mip_depth(p, verbose, submip, lp_probe_secs, 0);
        // 支配の固定の段まで来ずに終わった (途中で返った) のに消す候補があったなら、前処理の結果にもう一度かけて、そこで消す
        let held = INTSING_HELD.with(|h| h.get());
        let r = if INTSING_HOLD.with(|h| h.get()) == 1 && held > 0 {
            INTSING_HOLD.with(|h| h.set(2));
            if verbose {
                eprintln!("MIP: presolve: INTSING held {held} integer singleton candidates; presolving the result again (ENOMOTO_MIP_INTSING h)");
            }
            match r {
                Some(Presolved::Reduced { prob, postsolve, scaling }) => {
                    // かけ直しと同じく、プロービングと支配の固定はしない
                    DOMCOL_REPRESOLVING.with(|f| f.set(true));
                    let r2 = presolve_mip_depth(&prob, verbose, submip, None, 0);
                    DOMCOL_REPRESOLVING.with(|f| f.set(false));
                    match r2 {
                        Some(Presolved::Infeasible) => Some(Presolved::Infeasible),
                        Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) if sc2.d.iter().chain(&scaling.d).all(|&d| d == 1.0) => {
                            let mut postsolve = postsolve;
                            postsolve.extend(post2);
                            Some(Presolved::Reduced { prob: prob2, postsolve, scaling })
                        }
                        _ => Some(Presolved::Reduced { prob, postsolve, scaling }),
                    }
                }
                other => other,
            }
        } else {
            r
        };
        INTSING_HOLD.with(|h| h.set(0));
        return r;
    }
    presolve_mip_depth(p, verbose, submip, lp_probe_secs, 0)
}

/// 試作 `ENOMOTO_MIP_FOLD` の `P`・`Q` (前処理の後に畳む、[`fold::solve_folded_after_presolve`]) 用: 前処理をかけた問題と、その問題の解を
/// 元の問題の解に戻す関数。前処理で実行不能と分かったか、何も縮まなかったなら `None`。
pub(crate) type Unpresolve = Box<dyn Fn(&[f64]) -> Vec<f64>>;
pub(crate) fn presolve_for_fold(p: &MipProblem, verbose: bool) -> Option<(MipProblem, Unpresolve)> {
    match presolve_mip(p, verbose, false, None)? {
        Presolved::Infeasible => None,
        Presolved::Reduced { prob, postsolve, scaling } => {
            let n = p.n;
            let f = move |xr: &[f64]| -> Vec<f64> {
                let mut x = xr.to_vec();
                for step in postsolve.iter().rev() {
                    step.apply(&mut x);
                }
                let mut x = crate::presolve::scaling::unscale_x(&scaling, &x);
                x.truncate(n);
                x
            };
            Some((prob, Box::new(f)))
        }
    }
}

fn presolve_mip_depth(p: &MipProblem, verbose: bool, submip: bool, lp_probe_secs: Option<f64>, depth: u32) -> Option<Presolved> {
    use crate::params::simplex::{PRESOLVE_ROUNDS, PROPAGATION_PASSES, ROWSINGLETON_COLSINGLETON_INNER_ROUNDS};
    let _pt_nest = pretime::nest();
    let pt_build = pretime::guard("build");
    // 前の呼び出しのプロービングが残した同値な列の組は使わない (列の番号が違いうる)
    PROBE_EQUIV.with(|e| e.borrow_mut().clear());
    PROBE_EQUIV_FORCE.with(|f| f.set(false));
    let n = p.n;
    // 幅が許容誤差以下の列を 1 点に固定する ([`fix_tiny_width_columns`]。再スタートの問題の大域的な境界、プロービングで締めた境界を含む)
    let (mut col_lo, mut col_up) = (p.col_lo.clone(), p.col_up.clone());
    let (mut p_row_lo, mut p_row_up) = (p.row_lo.clone(), p.row_up.clone());
    let tiny0 = fix_tiny_width_columns(&p.rows, &mut p_row_lo, &mut p_row_up, &mut col_lo, &mut col_up, &p.is_int, submip, "pre0");
    // `ENOMOTO_MIP_DUALEQ=early` (試作、既定の `1` では使わない。PRESOLVE2): 前処理の前に、双対の議論で不等式行を等式にする
    // ([`dual_row_equalities`])。元の行で錠を数えるので、前処理で消える冗長な行も錠に数えてしまう (gt2)。ふつうは `=1` (前処理の最後)
    if dualeq() == Some("early") && !submip {
        let t0 = std::time::Instant::now();
        let k = dual_row_equalities(&p.rows, &mut p_row_lo, &mut p_row_up, &p.cols, &p.cost, &p.is_int, &col_lo, &col_up);
        if verbose {
            eprintln!("MIP: presolve: DUALEQ made {k} rows equations ({:.2}s)", t0.elapsed().as_secs_f64());
        }
    }
    // `ENOMOTO_MIP_DUAL_BOUND2` (試作): 双対の推論による境界の締め付け ([`dual_bound_tightening2`]、整数列も)
    if env_str!("ENOMOTO_MIP_DUAL_BOUND2").is_some() {
        let k = dual_bound_tightening2(&p.rows, &p_row_lo, &p_row_up, &p.cols, &p.cost, &p.is_int, &mut col_lo, &mut col_up);
        if verbose && k > 0 {
            eprintln!("MIP: presolve: tightened {k} column bounds by dual arguments (DUAL_BOUND2)");
        }
    }
    // 元の制約 (境界は丸め済みの MipProblem の値を使う)。行を `LinearExpr` (BTreeMap) に詰め直して範囲制約を 2 本に分けてから
    // `build_a_g` に渡す代わりに、行から直接作る ([`crate::presolve::build_a_g_from_ranged_rows`]、結果はビット単位で同じ)。
    // 行の列番号が狭義の昇順でないなどで作れなければ従来の経路で作る
    // 診断用 (`ENOMOTO_MIP_DEBUG_FASTPACK_CHECK`): 両方の経路で作り、ビット単位で同じかを確かめる
    let fastpack_check = env_str!("ENOMOTO_MIP_DEBUG_FASTPACK_CHECK").is_some();
    // `ENOMOTO_MIP_TREE2_FORCEQ` (2026-11-04 から既定、`=0` で使わない。[`tree2_forceq_rows`]): 0-1 列 1 本の強制等式から `x_j = v0 + d y` の 2 項等式を足す
    let forceq: Vec<(Vec<(usize, f64)>, f64)> = if env_def!("ENOMOTO_MIP_TREE2_FORCEQ", "1").is_some() {
        let t0 = std::time::Instant::now();
        let r = tree2_forceq_rows(&p.rows, &p_row_lo, &p_row_up, &col_lo, &col_up, &p.is_int);
        if verbose {
            eprintln!("MIP: presolve: TREE2_FORCEQ added {} doubleton equalities from forcing rows ({:.2}s)", r.len(), t0.elapsed().as_secs_f64());
        }
        r
    } else {
        Vec::new()
    };
    let aug_rows: std::borrow::Cow<'_, Vec<Vec<(usize, f64)>>> = if forceq.is_empty() {
        std::borrow::Cow::Borrowed(&p.rows)
    } else {
        let mut r = p.rows.clone();
        for (row, rhs) in &forceq {
            r.push(row.clone());
            p_row_lo.push(*rhs);
            p_row_up.push(*rhs);
        }
        std::borrow::Cow::Owned(r)
    };
    let fast = crate::presolve::build_a_g_from_ranged_rows(&col_lo, &col_up, &aug_rows, &p_row_lo, &p_row_up);
    let fast_ok = fast.is_some();
    let (a, b, g, h) = match fast.filter(|_| !fastpack_check) {
        Some(r) => r,
        None => {
            let vars: Vec<VariableData> = (0..n).map(|j| VariableData { vtype: if p.is_int[j] { VarType::Integer } else { VarType::Continuous }, lb: col_lo[j], ub: col_up[j] }).collect();
            let cons = ranged_constraints((0..aug_rows.len()).map(|i| (&aug_rows[i][..], p_row_lo[i], p_row_up[i])), aug_rows.len());
            let r = crate::presolve::build_a_g(&vars, &cons);
            if fastpack_check && fast_ok {
                let f = crate::presolve::build_a_g_from_ranged_rows(&col_lo, &col_up, &aug_rows, &p_row_lo, &p_row_up).unwrap();
                let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<u64>>();
                let mat = |m: &crate::sparse::FaerCsr| (m.nrows(), m.ncols(), crate::sparse::csr_rows(m).into_iter().map(|r| r.into_iter().map(|(j, v)| (j, v.to_bits())).collect::<Vec<_>>()).collect::<Vec<_>>());
                let same = mat(&f.0) == mat(&r.0) && bits(&f.1) == bits(&r.1) && mat(&f.2) == mat(&r.2) && bits(&f.3) == bits(&r.3);
                eprintln!("MIP: presolve: FASTPACK_CHECK {} (rows {}, eq {}, ineq {})", if same { "same" } else { "DIFFERENT" }, p.m, r.0.nrows(), r.2.nrows());
            }
            r
        }
    };
    drop(pt_build);
    let pt_red = pretime::guard("reduce");
    let mut pre = crate::presolve::run_extended_mip(n, &a, &b, &g, &h, &p.cost, &p.is_int, PROPAGATION_PASSES, PRESOLVE_ROUNDS, ROWSINGLETON_COLSINGLETON_INNER_ROUNDS);
    drop(pt_red);
    if pre.infeasible {
        return Some(Presolved::Infeasible);
    }
    if pre.unbounded {
        return None;
    }
    let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut row_lo = Vec::new();
    let mut row_up = Vec::new();
    for (i, r) in crate::sparse::csr_rows(&pre.a).into_iter().enumerate() {
        if r.is_empty() {
            continue;
        }
        rows.push(r);
        row_lo.push(pre.b[i]);
        row_up.push(pre.b[i]);
    }
    for (r, &rhs) in pre.real_rows.iter().zip(&pre.real_rhs) {
        if r.is_empty() {
            continue;
        }
        rows.push(r.clone());
        row_lo.push(f64::NEG_INFINITY);
        row_up.push(rhs);
    }
    // 前処理 (伝播・行シングルトンなど) が締めた境界でも、幅が許容誤差以下になった列を固定する (係数の強化の前)
    let tiny1 = fix_tiny_width_columns(&rows, &mut row_lo, &mut row_up, &mut pre.lb, &mut pre.ub, &p.is_int, submip, "pre1");
    if verbose && tiny0 + tiny1 > 0 {
        eprintln!("MIP: presolve: fixed {} columns of width within tolerance ({tiny0} before, {tiny1} after the reductions)", tiny0 + tiny1);
    }
    // 係数の強化 (片側の行の 0-1 列の係数を、整数解を変えずに LP の緩和が締まる値に縮める)
    let pt_coef = pretime::guard("coef_merge");
    if env_str!("ENOMOTO_MIP_NO_COEF_TIGHTEN").is_none() {
        let is_int_now: Vec<bool> = p.is_int.clone();
        COEF_TIGHTEN_SKIPPED.with(|c| c.set(0));
        let tightened = tighten_coefficients(&mut rows, &mut row_lo, &mut row_up, &pre.lb, &pre.ub, &is_int_now, env_str!("ENOMOTO_MIP_COEF_TIGHTEN_GENERAL").is_some());
        let skipped = COEF_TIGHTEN_SKIPPED.with(|c| c.get());
        let normalized = normalize_integer_rows(&mut rows, &mut row_lo, &mut row_up, &is_int_now);
        if verbose && (tightened > 0 || normalized > 0 || skipped > 0) {
            eprintln!("MIP: presolve: tightened {tightened} coefficients (skipped {skipped} in rows redundant within tolerance), normalized {normalized} integer rows");
        }
    }
    // 係数が定数倍の関係にある行をまとめる (上限側と下限側の組は範囲制約 1 本に、同じ向きはきつい方に)。
    // 前処理は範囲制約を 2 本の不等式で表すので、そのままだと LP の行が倍になる
    let eq0 = (0..rows.len()).filter(|&i| row_lo[i] == row_up[i]).count();
    let merged = if env_str!("ENOMOTO_MIP_NO_ROW_MERGE").is_none() { merge_parallel_rows(&mut rows, &mut row_lo, &mut row_up) } else { 0 };
    // 行をまとめて新しくできた等式 (`x - y <= 0` と `x - y >= 0` から `x = y` など) の数
    let new_eq = (0..rows.len()).filter(|&i| row_lo[i] == row_up[i]).count().saturating_sub(eq0);
    drop(pt_coef);
    if verbose && merged > 0 {
        eprintln!("MIP: presolve: merged {merged} parallel rows ({new_eq} new equations)");
    }
    // `ENOMOTO_MIP_SPARSIFY`: 等式の倍数を引いて行を疎にする ([`sparsify_rows`])。書き換えたら前処理をかけ直す (下の
    // MERGE_REPRESOLVE と同じ形。疎になった行が冗長・強制になったり、列が支配されたりする)
    let sparsified = if env_str!("ENOMOTO_MIP_SPARSIFY").is_some() && !submip && !SPARSIFY_REPRESOLVING.with(|f| f.get()) {
        let k = if env_str!("ENOMOTO_MIP_SPARSIFY").is_some_and(|v| v == "highs") {
            sparsify_rows_highs(&mut rows, &mut row_lo, &mut row_up, &p.is_int, &pre.lb, &pre.ub, n)
        } else {
            sparsify_rows(&mut rows, &mut row_lo, &mut row_up, &p.is_int, n)
        };
        if verbose {
            eprintln!("MIP: presolve: sparsified {k} rows");
        }
        k
    } else {
        0
    };
    let fixed = (0..n).filter(|&j| pre.lb[j] == pre.ub[j]).count();
    if verbose {
        let nnz0 = p.nnz;
        let nnz1: usize = rows.iter().map(|r| r.len()).sum();
        let par = |int: bool| pre.postsolve_log.iter().filter(|s| matches!(s, crate::presolve::PostsolveStep::ParallelCol(c) if p.is_int[c.var] == int)).count();
        eprintln!(
            "MIP: presolve: rows {} -> {}, free columns {} -> {}, nonzeros {nnz0} -> {nnz1}, postsolve steps {} (parallel columns merged: {} integer, {} continuous)",
            p.m,
            rows.len(),
            (0..n).filter(|&j| p.col_lo[j] < p.col_up[j]).count(),
            n - fixed,
            pre.postsolve_log.len(),
            par(true),
            par(false)
        );
    }
    // 代入消去で目的関数に生じた定数を offset に入れる (後処理は縮約後の点について
    // アフィンなので、任意の 1 点で元の目的値との差を測ればよい)。定数が抜けていると
    // 相対ギャップによる終了判定や目的値の刻みによる打ち切りが誤ったスケールで働く。
    let x_red: Vec<f64> = (0..n).map(|j| 0.0f64.clamp(pre.lb[j].min(pre.ub[j]), pre.ub[j].max(pre.lb[j]))).collect();
    let mut x_orig = x_red.clone();
    for step in pre.postsolve_log.iter().rev() {
        step.apply(&mut x_orig);
    }
    let x_orig = crate::presolve::scaling::unscale_x(&pre.scaling, &x_orig);
    let shift = p.objective(&x_orig) - (pre.c.iter().zip(&x_red).map(|(c, x)| c * x).sum::<f64>() + p.offset);
    let offset = if shift.is_finite() { p.offset + shift } else { p.offset };
    let mut prob = MipProblem::from_rows(pre.lb.clone(), pre.ub.clone(), pre.c.clone(), offset, p.sense_sign, p.is_int.clone(), rows, row_lo, row_up);
    // 診断 `ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED=接頭辞` (振る舞いは変えない): 前処理の後の問題を `接頭辞_番号.mps` に書く (サブ MIP は除く)
    if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED").filter(|_| !submip) {
        static CNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = format!("{f}_{}.mps", CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
        dump_mps(&prob, &id);
        eprintln!("MIP: presolve: dumped the presolved problem to {id} (offset {})", prob.offset);
    }
    // `ENOMOTO_MIP_KEEP_IMPLICATIONS`: 元の問題が持っていた含意 (サブ MIP なら親のプロービングの結果) を引き継ぐ。前処理は
    // 列の番号を変えないので、両端が前処理後も尺度 1 の 0-1 列のままの組だけ残す (HiGHS もサブ MIP に含意とクリーク表を渡す)
    // `ENOMOTO_MIP_CLQPRB` ([`clqprb_on`]) のときも、プロービングの後の前処理のかけ直し (`depth` > 0) では引き継ぐ (次のプロービングが
    // 前のプロービングの含意を伝播に使う。HiGHS はクリーク表を前処理の繰り返しの間で持ち続ける)
    // `g`・`r`・`s` のときは、含意を使う形に切り替えた求解 (`CLQPRB_STICKY`) だけで引き継ぐ。既定の形のままの問題・既定の形に戻した問題
    // (`CLQPRB_REVERTED`) で引き継ぐと、前処理が既定と変わる (gate2 clqprb_s: snp-02-004-104・supportcase40 で見つけた漏れ)
    let clqprb_live = clqprb_on() && !CLQPRB_REVERTED.with(|f| f.get()) && (!clqprb_has('g') || CLQPRB_STICKY.with(|f| f.get()));
    // `ENOMOTO_MIP_PRBWORK` ([`prbwork_on`]): 再スタートの後でプロービングを回さないときは、前の求解の含意 (再スタートの問題に渡したもの) を引き継ぐ
    let keep_impl = env_str!("ENOMOTO_MIP_KEEP_IMPLICATIONS").is_some() || (clqprb_live && depth > 0 && !submip) || (RESTART_PRESOLVE.with(|f| f.get()) && prbwork_skip_restart(submip));
    if keep_impl && !p.conflict_edges.is_empty() {
        let bin = |j: usize| prob.is_int[j] && prob.col_lo[j] == 0.0 && prob.col_up[j] == 1.0 && pre.scaling.d.get(j).is_none_or(|&d| d == 1.0);
        prob.conflict_edges = p.conflict_edges.iter().copied().filter(|&(a, b)| bin((a / 2) as usize) && bin((b / 2) as usize)).collect();
    }
    // 行をまとめた後のかけ直し (既定で使う。`ENOMOTO_MIP_NO_MERGE_REPRESOLVE` で使わない): 行をまとめて等式ができたら、前処理をもう一度かける (1 回だけ)。前処理の中では向きの逆の
    // 不等式の組は等式として見えないので、`x = y`・`x + y = 1` の代入 (二項等式の消去) がされずに残る。physiciansched6-2 は
    // こうしてできた 0-1 列どうしの `x - y = 0` が 3482 本、`x + y = 1` が 46 本残り (HiGHS の前処理後は等式 283 本)、
    // Feasibility Jump が解を見つけられない。後処理は内側の手順の後に外側の手順を並べる (LP による固定と同じ)
    let merge_rp = new_eq > 0 && env_str!("ENOMOTO_MIP_NO_MERGE_REPRESOLVE").is_none() && !MERGE_REPRESOLVING.with(|f| f.get());
    if merge_rp || sparsified > 0 {
        // 疎化のかけ直しの中では疎化しない (1 回だけ)
        let (was_m, was_s) = (MERGE_REPRESOLVING.with(|f| f.get()), SPARSIFY_REPRESOLVING.with(|f| f.get()));
        MERGE_REPRESOLVING.with(|f| f.set(was_m || merge_rp));
        SPARSIFY_REPRESOLVING.with(|f| f.set(was_s || sparsified > 0));
        let r = presolve_mip(&prob, verbose, submip, lp_probe_secs);
        MERGE_REPRESOLVING.with(|f| f.set(was_m));
        SPARSIFY_REPRESOLVING.with(|f| f.set(was_s));
        match r {
            Some(Presolved::Infeasible) => return Some(Presolved::Infeasible),
            Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) if sc2.d.iter().chain(&pre.scaling.d).all(|&d| d == 1.0) => {
                let mut postsolve = pre.postsolve_log;
                postsolve.extend(post2);
                return Some(Presolved::Reduced { prob: prob2, postsolve, scaling: pre.scaling });
            }
            // 何も減らない (または尺度が掛かった): このまま続ける
            _ => {}
        }
    }
    // `ENOMOTO_MIP_TIGHTEN_VUBS` (試作、既定では使わない): VUB の行の連続列の境界を LP で締め ([`tighten_vubs`])、締めたら
    // 前処理をかけ直す (係数の強化が `x <= M y` の `M` を締めた境界まで縮める)。根の問題で 1 回だけ (再スタートでは使わない)
    if lp_probe_secs.is_some() && !submip && env_str!("ENOMOTO_MIP_TIGHTEN_VUBS").is_some() && !VUBS_ACTIVE.with(|f| f.get()) && !RESTART_PRESOLVE.with(|f| f.get()) {
        match tighten_vubs(&mut prob, verbose) {
            None => return Some(Presolved::Infeasible),
            Some(k) if k > 0 => {
                let mut postsolve = pre.postsolve_log;
                VUBS_ACTIVE.with(|f| f.set(true));
                let r = presolve_mip_depth(&prob, verbose, submip, lp_probe_secs, depth);
                VUBS_ACTIVE.with(|f| f.set(false));
                return match r {
                    Some(Presolved::Infeasible) => Some(Presolved::Infeasible),
                    Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) => {
                        debug_assert!(sc2.d.iter().chain(&pre.scaling.d).all(|&d| d == 1.0));
                        postsolve.extend(post2);
                        Some(Presolved::Reduced { prob: prob2, postsolve, scaling: pre.scaling })
                    }
                    None => Some(Presolved::Reduced { prob, postsolve, scaling: pre.scaling }),
                };
            }
            Some(_) => {}
        }
    }
    // 早めのプロービングとかけ直し (既定で使う。`ENOMOTO_MIP_NO_PROBE_EARLY_REPRESOLVE` で使わない。以前の試作名は
    // `ENOMOTO_MIP_PROBE_EARLY_REPRESOLVE`): LP による固定の前に、少しだけプロービングし ([`quick_probe_fixings`])、
    // 列の多くが固定されたらその境界で前処理をかけ直す (LP による固定・プロービングは小さくなった問題で行う)。
    // 手元 16 スレッド・60 秒: neos8 16.2 -> 6.6 秒、drayage-100-23 5.9 -> 3.0 秒 (analysis/eprostop_16t_20261028.md)
    // `ENOMOTO_MIP_CLQACT` (試作): 最初の前処理の後、プロービング・LP による固定の前に、行の活動量のクリーク分割で係数と境界を締める
    // ([`clqact_tighten`])。LP による固定の後にかけると、その LP の基底を根で使えなくなる (uccase9 で根の LP が 130 → 16462 反復)
    if clqact_on() && lp_probe_secs.is_some() && !submip && depth == 0 && !RESTART_PRESOLVE.with(|f| f.get()) && !QUICK_PROBE_ACTIVE.with(|f| f.get()) {
        clqact_tighten(&mut prob, verbose);
    }
    if env_str!("ENOMOTO_MIP_DEBUG_INTCENSUS").is_some() && lp_probe_secs.is_some() && !submip && depth == 0 && !RESTART_PRESOLVE.with(|f| f.get()) && !QUICK_PROBE_ACTIVE.with(|f| f.get()) {
        int_census(&prob, &prob.col_lo, &prob.col_up, None, "a_presolve");
    }
    if lp_probe_secs.is_some() && !submip && env_str!("ENOMOTO_MIP_NO_PROBE_EARLY_REPRESOLVE").is_none() && !QUICK_PROBE_ACTIVE.with(|f| f.get()) {
        match quick_probe_fixings(&mut prob, verbose, submip) {
            None => return Some(Presolved::Infeasible),
            Some(k) if k > 0 => {
                let mut postsolve = pre.postsolve_log;
                QUICK_PROBE_ACTIVE.with(|f| f.set(true));
                let r = presolve_mip_depth(&prob, verbose, submip, lp_probe_secs, depth);
                QUICK_PROBE_ACTIVE.with(|f| f.set(false));
                return match r {
                    Some(Presolved::Infeasible) => Some(Presolved::Infeasible),
                    Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) => {
                        debug_assert!(sc2.d.iter().chain(&pre.scaling.d).all(|&d| d == 1.0));
                        postsolve.extend(post2);
                        Some(Presolved::Reduced { prob: prob2, postsolve, scaling: pre.scaling })
                    }
                    // かけ直しが何も減らさない: 固定した境界を持つ問題をそのまま使う
                    None => Some(Presolved::Reduced { prob, postsolve, scaling: pre.scaling }),
                };
            }
            Some(_) => {}
        }
    }
    // 試作 `ENOMOTO_MIP_PRECHAIN` ([`prechain::start`]): 軽い縮約 (行をまとめた後のかけ直し・早めのプロービングとかけ直しを含む) の後、
    // LP による固定・プロービングなどの重い前処理の前に、FJ・Local-MIP のスレッドを立てる (最初の求解で 1 回だけ)。`p` なら並列の
    // プロービングと分け合う。立てた後にかけ直しの前処理がここを通ったら、縮んだ問題に乗り換えさせる ([`prechain::refresh`])
    // サブ MIP の前処理では呼ばない (根の後も連鎖が生きている `r`・`R` で、主のスレッドで解くサブ MIP の前処理がサブ MIP の問題を
    // 渡して連鎖を乗り換えさせていた。analysis/restchain_20261111.md)
    if !submip && prechain::on() && !PRESOLVE_IN_SUBMIP.with(|f| f.get()) {
        match prechain::start(&prob) {
            Some(par) => PREPAR_THREADS.with(|t| t.set(par)),
            None => prechain::refresh(&prob),
        }
    }
    // LP による固定は含意を集めるプロービングの前に行う (固定をプロービングの伝播に使い、固定した列の含意を作らない)。
    // 固定したら前処理をもう一度かける (固定した列と、それで冗長になった行を消し、縮約を続ける。プロービングもそちらで行う)。
    // 前処理は列の番号を保ち、整数最適化では列の尺度をかけない (d = 1) ので、後処理は内側の手順の後に外側の手順を並べればよい
    if lp_probe_secs.is_some() && !submip && heuristics::fj_feas_extend(&prob) {
        if let Some((threads, cap)) = FEAS_FJ_EARLY.with(|c| c.take()) {
            pretime::mark("fj_early_start");
            let pt_fj = pretime::guard("fj_early");
            let r = if feas_fj_early_lmip() { feas_fj_lmip_early(&prob, threads, cap, verbose) } else { feas_fj_early(&prob, threads, cap, verbose) };
            drop(pt_fj);
            pretime::mark("fj_early_end");
            if let Some(x) = r {
                FEAS_FJ_SOL.with(|s| *s.borrow_mut() = Some(x));
                return Some(Presolved::Reduced { prob, postsolve: pre.postsolve_log, scaling: pre.scaling });
            }
        }
    }
    // `ENOMOTO_MIP_VBCLQ` の `e` (既定、`=0` で使わない。[`presolve_vbclq_e`]、analysis/scipgate_20261120): LP による固定の前に、暫定解の値を上限にした
    // pseudoobj の固定をかける。固定したら前処理をかけ直し (固定した列と冗長になった行を消す)、その問題で LP による固定を回す
    // 診断 `R` (値 `eR`): 前処理の中では回さず、根の LP の前の道 (前処理で回らなかった求解の道) だけで回す (fuzz でその道を確かめる)
    if lp_probe_secs.is_some() && !submip && !VBCLQ_E_ACTIVE.with(|f| f.get()) && solver::vbclq().is_some_and(|v| v.contains('e') && !v.contains('R')) {
        // 固定の前の問題を取っておく: 固定の後のかけ直しが実行不能と言ったら、それは信じずに (VB01 の解は許容誤差 1e-6 の内で実行可能で、
        // 厳密な最適より良い値のことがある。その値を上限にした固定で厳密に実行可能な点が全て消える: fuzz_tiny の種 64・362)、
        // 固定の前の問題で続ける (VB01 の解は開始解としてそのまま渡す)
        let saved = prob.clone();
        let k = presolve_vbclq_e(&mut prob, verbose, lp_probe_secs.unwrap_or(10.0));
        VBCLQ_E_RAN.with(|f| f.set(true));
        if k == usize::MAX {
            // 最適と証明済み: 早い FJ の解と同じく、前処理の残り・クリーク行の拡張・対称性・木を飛ばす
            return Some(Presolved::Reduced { prob, postsolve: pre.postsolve_log, scaling: pre.scaling });
        }
        if k > 0 {
            VBCLQ_E_ACTIVE.with(|f| f.set(true));
            let r = presolve_mip_depth(&prob, verbose, submip, lp_probe_secs, depth);
            VBCLQ_E_ACTIVE.with(|f| f.set(false));
            match r {
                Some(Presolved::Infeasible) => {
                    if verbose {
                        eprintln!("MIP: VBCLQ e: the presolve after the fixings says infeasible; the fixings are dropped");
                    }
                    prob = saved;
                }
                Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) => {
                    debug_assert!(sc2.d.iter().chain(&pre.scaling.d).all(|&d| d == 1.0));
                    let mut postsolve = pre.postsolve_log;
                    postsolve.extend(post2);
                    return Some(Presolved::Reduced { prob: prob2, postsolve, scaling: pre.scaling });
                }
                None => return Some(Presolved::Reduced { prob, postsolve: pre.postsolve_log, scaling: pre.scaling }),
            }
        }
    }
    if let Some(secs) = lp_probe_secs {
        match lp_probe(&mut prob, verbose, secs, true) {
            None => return Some(Presolved::Infeasible),
            Some(fixed) if fixed > 0 && env_str!("ENOMOTO_MIP_LP_PROBE_NO_REPRESOLVE").is_none() => {
                let mut postsolve = pre.postsolve_log;
                return match presolve_mip_depth(&prob, verbose, submip, None, depth) {
                    Some(Presolved::Infeasible) => Some(Presolved::Infeasible),
                    Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) => {
                        debug_assert!(sc2.d.iter().chain(&pre.scaling.d).all(|&d| d == 1.0));
                        postsolve.extend(post2);
                        Some(Presolved::Reduced { prob: prob2, postsolve, scaling: pre.scaling })
                    }
                    // 2 回目の前処理が何も減らさない: 1 回目の結果にプロービングだけかける
                    None => {
                        if env_str!("ENOMOTO_MIP_NO_PROBING").is_none() {
                            match probe(&mut prob, verbose, submip) {
                                Some(false) => return Some(Presolved::Infeasible),
                                Some(true) if !probe_equiv_fix() => return represolve_after_probe(prob, postsolve, pre.scaling, verbose, submip, depth),
                                _ => {}
                            }
                        }
                        if PROBE_EQUIV_FORCE.with(|f| f.get()) {
                            return match substitute_probe_equiv(&prob, postsolve, pre.scaling, verbose, submip) {
                                Ok(r) => Some(r),
                                Err((postsolve, scaling)) => Some(Presolved::Reduced { prob, postsolve, scaling }),
                            };
                        }
                        Some(Presolved::Reduced { prob, postsolve, scaling: pre.scaling })
                    }
                };
            }
            Some(_) => {}
        }
    }
    // 試作 `ENOMOTO_MIP_PRR_PRECHECK` (既定では使わない、analysis/slow2_20261117): プロービングの後のかけ直しの前処理で、
    // プロービングの前の時点で行が捨てる割合ほど減っていなければ、ここで返す (呼び手 [`represolve_after_probe`] は行の減り方を見て
    // 結果を捨てる)。mzzv42z ではかけ直しが 10460 -> 10039 -> 10006 行 (4%) で捨てられ、中の 2 回のプロービングに 10.7 秒使っていた
    if let Some((d, m_in, frac)) = PRR_PRECHECK.with(|c| c.get()) {
        if d == depth && (m_in as f64 - prob.m as f64) < frac * m_in as f64 {
            if verbose {
                eprintln!("MIP: presolve again after probing: rows {m_in} -> {} before probing (less than {:.0}% fewer), probing skipped (ENOMOTO_MIP_PRR_PRECHECK)", prob.m, 100.0 * frac);
            }
            return Some(Presolved::Reduced { prob, postsolve: pre.postsolve_log, scaling: pre.scaling });
        }
    }
    // `ENOMOTO_MIP_DOMCOL_NOPROBE2` (2026-11-18 から既定、`=0` で止める): `ENOMOTO_MIP_DOMCOL` の固定の後のかけ直しではプロービングをしない。プロービングは
    // 固定の前に済んでいて、air03 では 2297 列の固定の後のかけ直しで 2.8 秒のプロービングをやり直し (固定 0)、4.5 → 6.8 秒になっていた
    let skip_probe2 = DOMCOL_REPRESOLVING.with(|f| f.get()) && env_def!("ENOMOTO_MIP_DOMCOL_NOPROBE2", "1").is_some();
    if env_str!("ENOMOTO_MIP_NO_PROBING").is_none() && !skip_probe2 {
        match probe(&mut prob, verbose, submip) {
            Some(false) => return Some(Presolved::Infeasible),
            Some(true) if !probe_equiv_fix() => return represolve_after_probe(prob, pre.postsolve_log, pre.scaling, verbose, submip, depth),
            _ => {}
        }
    }
    // `ENOMOTO_MIP_DOMCOL` (2026-11-18 から既定、`=0` で止める。docs/improvement_history.md): 支配される 0-1 列を固定し ([`dominated_binary_fixings`])、前処理をかけ直す (LP による固定と同じ形、1 回だけ)
    if env_def!("ENOMOTO_MIP_DOMCOL", "1").is_some() && !submip && !DOMCOL_REPRESOLVING.with(|f| f.get()) {
        let t0 = std::time::Instant::now();
        let mut fix = dominated_binary_fixings(&prob);
        if verbose {
            eprintln!("MIP: presolve: {} dominated binaries fixed ({:.2}s)", fix.len(), t0.elapsed().as_secs_f64());
        }
        // 診断 `ENOMOTO_MIP_DIAG_COSTSHIFT` (analysis/slowm_20261119 §14、振る舞いは変えない): 等式でずらした費用 c' = c − λᵀA_E で固定が増えるか数える
        if env_str!("ENOMOTO_MIP_DIAG_COSTSHIFT").is_some() {
            diag_costshift(&prob, fix.len());
        }
        // `ENOMOTO_T_MIP_DOMCOL_MIN_FRAC` (2026-11-18 から既定 0.1、`=0` で使わない): 固定が自由な列のこの割合に満たなければ固定を捨てる。drev の P
        // (analysis/vsscip_slow_20261118/tree2/README.md 4 節) で遅くなった nw04 (3440 / 87482 列 = 3.9%、2.35 倍)・nu25-pr12 (3.6%、1.58 倍)・
        // fiber (2.4%、1.35 倍) は固定が少なく、効いた rail507・fast0507 (58%)・air03 (21%) と分かれる
        let min_frac = tunable!("ENOMOTO_T_MIP_DOMCOL_MIN_FRAC", 0.1, f64);
        if min_frac > 0.0 && !fix.is_empty() {
            let nfree0 = (0..n).filter(|&j| prob.col_lo[j] < prob.col_up[j]).count();
            if (fix.len() as f64) < min_frac * nfree0 as f64 {
                if verbose {
                    eprintln!("MIP: presolve: the dominated binary fixings dropped ({} < {min_frac} x {nfree0} free columns)", fix.len());
                }
                fix.clear();
            }
        }
        for &(j, v) in &fix {
            prob.col_lo[j] = v;
            prob.col_up[j] = v;
        }
        // 固定が自由な列の `ENOMOTO_T_MIP_DOMCOL_REPRESOLVE_FRAC` (既定 5%) 未満なら、かけ直さずに境界だけ固定する
        // (nw04 は 87482 列中 372 列の固定に前処理 1 回 (約 1.8 秒) をかけ直していた)
        let nfree = (0..n).filter(|&j| prob.col_lo[j] < prob.col_up[j]).count() + fix.len();
        // INTSING の `h`: 保留していた (状態 1) なら解き放ち (状態 2)、消す候補があったなら固定が少なくてもかけ直して、そこで消す
        let intsing_release = crate::presolve::colsingleton::INTSING_HOLD.with(|h| h.get()) == 1 && {
            crate::presolve::colsingleton::INTSING_HOLD.with(|h| h.set(2));
            let held = crate::presolve::colsingleton::INTSING_HELD.with(|h| h.get());
            if verbose {
                eprintln!("MIP: presolve: INTSING held {held} integer singleton candidates until the dominated binary fixings (ENOMOTO_MIP_INTSING h)");
            }
            held > 0
        };
        if intsing_release || (!fix.is_empty() && fix.len() as f64 >= tunable!("ENOMOTO_T_MIP_DOMCOL_REPRESOLVE_FRAC", 0.05, f64) * nfree as f64) {
            DOMCOL_REPRESOLVING.with(|f| f.set(true));
            let r = presolve_mip(&prob, verbose, submip, None);
            DOMCOL_REPRESOLVING.with(|f| f.set(false));
            match r {
                Some(Presolved::Infeasible) => return Some(Presolved::Infeasible),
                Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) if sc2.d.iter().chain(&pre.scaling.d).all(|&d| d == 1.0) => {
                    let mut postsolve = pre.postsolve_log;
                    postsolve.extend(post2);
                    return Some(Presolved::Reduced { prob: prob2, postsolve, scaling: pre.scaling });
                }
                _ => {}
            }
        }
    }
    match substitute_probe_equiv(&prob, pre.postsolve_log, pre.scaling, verbose, submip) {
        Ok(r) => Some(r),
        Err((postsolve, scaling)) => Some(Presolved::Reduced { prob, postsolve, scaling }),
    }
}

/// `ENOMOTO_MIP_VUBAGG` (既定で使う) の双対の集約 (SCIP の setppc・linear の双対の前処理に当たる。neos-787933 で SCIP は 236376 列を 1764 列にして根で解く)。
/// 行 `Σ a_j x_j − M y ≤ 0` (≥ の形は符号を反転して同じ。右辺 0、y は 0-1、x_j は全て 0-1 で a_j > 0、`Σ a_j ≤ M`) は、y = 0 なら x = 0、
/// y = 1 なら冗長なので、`x_j ≤ y` (j ごと) と同じ。x_j の費用が 0 以下で、x_j を増やして苦しくなる行 (上向きの錠) がこの行だけなら、
/// x_j を y まで上げても実行可能で目的は悪くならないので、`x_j = y` の最適解がある (列ごとに独立に上げてよい)。組 `(x_j, y, false)` を返す。
/// 同値な 0-1 列の組と同じ形 ([`PROBE_EQUIV`])。
fn vub_dual_agg_pairs(prob: &MipProblem) -> Vec<(usize, usize, bool)> {
    let binary = |j: usize| prob.is_int[j] && prob.col_lo[j] == 0.0 && prob.col_up[j] == 1.0;
    // 列ごとの上向きの錠の数 (係数 > 0 で上側が有限、係数 < 0 で下側が有限)
    let mut uplocks = vec![0u32; prob.n];
    for (i, r) in prob.rows.iter().enumerate() {
        for &(j, a) in r {
            if (a > 0.0 && prob.row_up[i].is_finite()) || (a < 0.0 && prob.row_lo[i].is_finite()) {
                uplocks[j] += 1;
            }
        }
    }
    let mut out = Vec::new();
    let mut used = vec![false; prob.n];
    for (i, r) in prob.rows.iter().enumerate() {
        if r.len() < 2 {
            continue;
        }
        // 向き: ≤ 0 なら sg = 1、≥ 0 なら sg = −1 (両側が有限な行は使わない)
        let sg = if prob.row_up[i] == 0.0 && prob.row_lo[i] == f64::NEG_INFINITY {
            1.0
        } else if prob.row_lo[i] == 0.0 && prob.row_up[i] == f64::INFINITY {
            -1.0
        } else {
            continue;
        };
        let neg: Vec<(usize, f64)> = r.iter().filter(|&&(_, a)| sg * a < 0.0).copied().collect();
        if neg.len() != 1 || !binary(neg[0].0) {
            continue;
        }
        let (y, m) = (neg[0].0, -sg * neg[0].1);
        if !r.iter().all(|&(j, a)| j == y || (sg * a > 0.0 && binary(j))) {
            continue;
        }
        let sum: f64 = r.iter().filter(|&&(j, _)| j != y).map(|&(_, a)| sg * a).sum();
        if sum > m * (1.0 + 1e-12) + 1e-9 {
            continue;
        }
        for &(j, _) in r {
            if j != y && !used[j] && prob.cost[j] <= 0.0 && uplocks[j] == 1 {
                used[j] = true;
                out.push((j, y, false));
            }
        }
    }
    // y が別の組の x なら、その組は使わない (代入の連鎖・輪を作らない)
    out.retain(|&(_, y, _)| !used[y]);
    out
}

/// `ENOMOTO_MIP_VUBAGG` (既定で使う): 組 `x_j = y` ([`vub_dual_agg_pairs`]) で x_j を全ての行から y に置き換え、x_j の費用を y に足し、x_j を 0 に固定した問題。
/// 解は `x_j = x_y` で戻す。列の番号は変えない。
/// `ENOMOTO_MIP_ORBPP_REPROBE` (既定で使う。`=0` で使わない): 静的なオービトープ (`ENOMOTO_MIP_ORBPP`) の固定の後に、(1) 行の伝播、(2) 静的なオービトープの固定
/// ([`symmetry::orbitopal_fixing`]、詰め込みの固定を含む)、(3) 自由な 0-1 列のプロービング (片側が行の伝播で矛盾すれば反対側に固定) を、
/// 何も変わらなくなるか `ENOMOTO_T_MIP_ORBPP_REPROBE_ROUNDS` (5) 回まで繰り返し、境界を問題に書き込む。どれも主の推論 (実行可能な点を消さない)
/// なので、木のオービトープの固定 (列の対応) はそのまま使える (列のまとめ・置き換え・双対の縮小はしない)。graph20-20-1rand では SCIP が
/// 双対の前処理を全部切っても対称性の後のプロービング (164 列) と logicor の伝播 (458 列) で 2183 → 326 列、最初の根の LP −12 (当方 −25.7)。
/// プロービングの手間は伝播で走査した行の長さの合計で `ENOMOTO_T_MIP_ORBPP_REPROBE_WORK` (200) x 非零まで。
/// 戻り値は [回数, 伝播の固定, オービトープの固定, プロービングの固定, 前の自由な 0-1 列, 後の自由な 0-1 列]
fn orbpp_reprobe(prob: &mut MipProblem, orbs: &[symmetry::Orbitope]) -> [usize; 6] {
    let bin = |p: &MipProblem, j: usize| p.is_int[j] && p.col_lo[j] >= 0.0 && p.col_up[j] <= 1.0;
    let free_bin = |p: &MipProblem| (0..p.n).filter(|&j| bin(p, j) && p.col_lo[j] < p.col_up[j]).count();
    let mut st = [0usize; 6];
    st[4] = free_bin(prob);
    let max_rounds = tunable!("ENOMOTO_T_MIP_ORBPP_REPROBE_ROUNDS", 5usize, usize);
    let rows_only = env_str!("ENOMOTO_MIP_ORBPP_REPROBE") == Some("r");
    let work_cap = (tunable!("ENOMOTO_T_MIP_ORBPP_REPROBE_WORK", 200.0, f64) * prob.nnz.max(1) as f64) as u64;
    let mut dom = domain::Domain::new(prob);
    let w0 = dom.debug_work();
    let count_fixed = |d: &domain::Domain, p: &MipProblem| (0..p.n).filter(|&j| bin(p, j) && d.lo[j] == d.up[j]).count();
    for _ in 0..max_rounds {
        st[0] += 1;
        let before = count_fixed(&dom, prob);
        // (1) 行の伝播
        let f0 = count_fixed(&dom, prob);
        if !dom.propagate(prob) {
            return st;
        }
        st[1] += count_fixed(&dom, prob) - f0;
        // (2) 静的なオービトープの固定 (変わらなくなるまで)
        loop {
            let mut fixes: Vec<(usize, f64)> = Vec::new();
            for o in orbs.iter().filter(|o| o.static_pp) {
                match symmetry::orbitopal_fixing(o, &dom.lo, &dom.up) {
                    None => return st,
                    Some(f) => fixes.extend(f),
                }
            }
            let f0 = count_fixed(&dom, prob);
            for (j, v) in fixes {
                dom.tighten_lower(prob, j, v);
                dom.tighten_upper(prob, j, v);
            }
            if dom.infeasible || !dom.propagate(prob) {
                return st;
            }
            let got = count_fixed(&dom, prob) - f0;
            st[2] += got;
            if got == 0 {
                break;
            }
        }
        // (3) プロービング (固定だけ)
        let f0 = count_fixed(&dom, prob);
        for j in 0..prob.n {
            if dom.debug_work() - w0 > work_cap {
                break;
            }
            if !bin(prob, j) || dom.lo[j] == dom.up[j] {
                continue;
            }
            let mut ok = [true; 2];
            for (k, v) in [0.0, 1.0].into_iter().enumerate() {
                let pos = dom.stack_len();
                dom.tighten_lower(prob, j, v);
                dom.tighten_upper(prob, j, v);
                // 行の伝播とオービトープの固定を、変わらなくなるまで交互に (`ENOMOTO_MIP_ORBPP_REPROBE=r` なら行の伝播だけ)。SCIP のプロービングも
                // orbitope_pp の伝播を含む (固定の後のプロービングで 164 列を固定する)
                ok[k] = !dom.infeasible && dom.propagate(prob);
                while ok[k] && !rows_only {
                    let mut fixes: Vec<(usize, f64)> = Vec::new();
                    for o in orbs.iter().filter(|o| o.static_pp) {
                        match symmetry::orbitopal_fixing(o, &dom.lo, &dom.up) {
                            None => {
                                ok[k] = false;
                                break;
                            }
                            Some(f) => fixes.extend(f.into_iter().filter(|&(c, v)| dom.lo[c] != v || dom.up[c] != v)),
                        }
                    }
                    if !ok[k] || fixes.is_empty() {
                        break;
                    }
                    for (c, v) in fixes {
                        dom.tighten_lower(prob, c, v);
                        dom.tighten_upper(prob, c, v);
                    }
                    ok[k] = !dom.infeasible && dom.propagate(prob);
                }
                dom.backtrack_to(prob, pos);
            }
            match ok {
                [false, false] => return st,
                [false, true] => {
                    dom.tighten_lower(prob, j, 1.0);
                }
                [true, false] => {
                    dom.tighten_upper(prob, j, 0.0);
                }
                _ => continue,
            }
            if dom.infeasible || !dom.propagate(prob) {
                return st;
            }
        }
        st[3] += count_fixed(&dom, prob) - f0;
        // 境界を問題に書き込む (伝播で締まった連続列・整数列の境界も)
        for j in 0..prob.n {
            prob.col_lo[j] = prob.col_lo[j].max(dom.lo[j]);
            prob.col_up[j] = prob.col_up[j].min(dom.up[j]);
        }
        if count_fixed(&dom, prob) == before || dom.debug_work() - w0 > work_cap {
            break;
        }
    }
    st[5] = free_bin(prob);
    // 固定で冗長になった行 (今の境界で最小・最大の活動量が両方とも行の範囲に入る行。列が全て固定された行を含む) を外す。graph20-20-1rand は
    // 自由な列 425 に対し行 4810 (非零 3220) が LP に残り、ノードの LP が全体の 2/3 を占めた。列の並びは変えないので、オービトープの列の対応はそのまま
    // (`ENOMOTO_MIP_ORBPP_REPROBE` の値に `k` があれば外さない)
    if !env_str!("ENOMOTO_MIP_ORBPP_REPROBE").is_some_and(|v| v.contains('k')) {
        let keep: Vec<bool> = (0..prob.m)
            .map(|i| {
                let (mut lo, mut hi) = (0.0f64, 0.0f64);
                for &(j, a) in &prob.rows[i] {
                    let (l, u) = (prob.col_lo[j], prob.col_up[j]);
                    if a > 0.0 {
                        lo += a * l;
                        hi += a * u;
                    } else {
                        lo += a * u;
                        hi += a * l;
                    }
                }
                let tol = 1e-9 * (1.0 + lo.abs().max(hi.abs()));
                !(lo.is_finite() && hi.is_finite() && lo >= prob.row_lo[i] - tol && hi <= prob.row_up[i] + tol)
            })
            .collect();
        let dropped = keep.iter().filter(|&&k| !k).count();
        if dropped > 0 {
            let rows: Vec<Vec<(usize, f64)>> = (0..prob.m).filter(|&i| keep[i]).map(|i| prob.rows[i].clone()).collect();
            let lo: Vec<f64> = (0..prob.m).filter(|&i| keep[i]).map(|i| prob.row_lo[i]).collect();
            let up: Vec<f64> = (0..prob.m).filter(|&i| keep[i]).map(|i| prob.row_up[i]).collect();
            let mut q = MipProblem::from_rows(prob.col_lo.clone(), prob.col_up.clone(), prob.cost.clone(), prob.offset, prob.sense_sign, prob.is_int.clone(), rows, lo, up);
            q.conflict_edges = std::mem::take(&mut prob.conflict_edges);
            q.implied_bounds = std::mem::take(&mut prob.implied_bounds);
            *prob = q;
        }
        st[0] += 1000 * dropped;
    }
    st
}

/// 試作 `ENOMOTO_MIP_BIGM_SPLIT`: 0-1 の列だけで係数が全て正の「≤」の行 `a y + Σ c_i x_i ≤ b` のうち、y = 0 なら冗長 (`Σ c_i ≤ b`)、
/// y = 1 なら他の全ての列が 0 (`c_i > b − a` かつ `a ≤ b`) になるもの (大 M の行) を、同じ実行可能集合の `x_i + y ≤ 1` (i ごと) に置き換える。
/// LP の緩和は強くなる (hypothyroid-k1 の 2512 行「45y + Σ x_i ≤ 45」。SCIP は knapsack の前処理で同じ分解をし、最初の LP 値 −2857 (最適 −2851)。
/// 当方の既定の根の LP は −2903)。足す非零が元の非零の `ENOMOTO_T_MIP_BIGM_SPLIT_NNZ` (1) 倍を超えるなら、行の短い順にそこまでにする。
/// 置き換える行がなければ None
fn bigm_split_problem(p: &MipProblem) -> Option<MipProblem> {
    let bin = |j: usize| p.is_int[j] && p.col_lo[j] >= 0.0 && p.col_up[j] <= 1.0;
    let mut cand: Vec<(usize, usize)> = Vec::new();
    for (i, r) in p.rows.iter().enumerate() {
        let b = p.row_up[i];
        if !b.is_finite() || p.row_lo[i].is_finite() || r.len() < 3 || !r.iter().all(|&(j, a)| a > 0.0 && bin(j)) {
            continue;
        }
        let (k, &(y, a)) = r.iter().enumerate().max_by(|x, z| x.1 .1.total_cmp(&z.1 .1)).unwrap();
        let tol = 1e-9 * b.abs().max(1.0);
        let rest: f64 = r.iter().enumerate().filter(|&(q, _)| q != k).map(|(_, &(_, c))| c).sum();
        let cmin = r.iter().enumerate().filter(|&(q, _)| q != k).map(|(_, &(_, c))| c).fold(f64::INFINITY, f64::min);
        if a <= b + tol && rest <= b + tol && cmin > b - a + tol {
            cand.push((i, y));
        }
    }
    if cand.is_empty() {
        return None;
    }
    let cap = tunable!("ENOMOTO_T_MIP_BIGM_SPLIT_NNZ", 1.0, f64) * p.nnz as f64;
    cand.sort_by_key(|&(i, _)| p.rows[i].len());
    let mut add = 0.0;
    let mut split: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for &(i, y) in &cand {
        let more = 2.0 * (p.rows[i].len() - 1) as f64;
        if add + more > cap {
            break;
        }
        add += more;
        split.insert(i, y);
    }
    if split.is_empty() {
        return None;
    }
    let (mut rows, mut lo, mut up) = (Vec::new(), Vec::new(), Vec::new());
    for (i, r) in p.rows.iter().enumerate() {
        match split.get(&i) {
            Some(&y) => {
                for &(j, _) in r.iter().filter(|&&(j, _)| j != y) {
                    let mut e = vec![(j, 1.0), (y, 1.0)];
                    e.sort_by_key(|&(c, _)| c);
                    rows.push(e);
                    lo.push(f64::NEG_INFINITY);
                    up.push(1.0);
                }
            }
            None => {
                rows.push(r.clone());
                lo.push(p.row_lo[i]);
                up.push(p.row_up[i]);
            }
        }
    }
    Some(MipProblem::from_rows(p.col_lo.clone(), p.col_up.clone(), p.cost.clone(), p.offset, p.sense_sign, p.is_int.clone(), rows, lo, up))
}

fn vub_dual_agg_problem(p: &MipProblem, pairs: &[(usize, usize, bool)]) -> MipProblem {
    let mut rep: Vec<usize> = (0..p.n).collect();
    for &(j, y, _) in pairs {
        rep[j] = y;
    }
    let mut cost = p.cost.clone();
    let (mut col_lo, mut col_up) = (p.col_lo.clone(), p.col_up.clone());
    for &(j, y, _) in pairs {
        cost[y] += cost[j];
        cost[j] = 0.0;
        col_lo[j] = 0.0;
        col_up[j] = 0.0;
    }
    let rows: Vec<Vec<(usize, f64)>> = p
        .rows
        .iter()
        .map(|r| {
            let mut m: std::collections::BTreeMap<usize, f64> = std::collections::BTreeMap::new();
            for &(j, a) in r {
                *m.entry(rep[j]).or_insert(0.0) += a;
            }
            m.into_iter().filter(|&(_, a)| a != 0.0).collect()
        })
        .collect();
    MipProblem::from_rows(col_lo, col_up, cost, p.offset, p.sense_sign, p.is_int.clone(), rows, p.row_lo.clone(), p.row_up.clone())
}

/// `ENOMOTO_MIP_PROBE_EQUIV2` の 2 巡目が多くの組を見つけたとき ([`PROBE_EQUIV_FORCE`]): プロービングが境界を締めたときも ([`probe`] が `Some(true)`)、
/// `ENOMOTO_MIP_PROBE_REPRESOLVE` がなければ [`represolve_after_probe`] で早く戻らず、後ろの同値な 0-1 列の代入
/// ([`substitute_probe_equiv`]) まで進む。既定では締めた境界が 1 本でもあると、見つけた同値な列を代入せずに返していた
/// (physiciansched6-2 は締め付け 179 本で、`ENOMOTO_MIP_PROBE_CLIQUE` の 1084 組を代入しない)。代入のかけ直しの前処理は
/// 締めた境界の上でかけるので、締めた境界も使われる。同値な組がなければ既定と同じ (早く戻るのと同じ結果になる)
/// (いつもそうする試作 `ENOMOTO_MIP_PROBE_EQUIV_FIX` は 2026-11-18 に削除)
fn probe_equiv_fix() -> bool {
    !prr_active(false)
        && PROBE_EQUIV.with(|e| !e.borrow().is_empty())
        && PROBE_EQUIV_FORCE.with(|f| f.get())
}

/// 同値な 0-1 列の代入 ([`presolve_mip`] の最後)。代入してかけ直した前処理が問題を減らしたらその結果を返す。
/// 減らなければ受け取った後処理と尺度を返す。
fn substitute_probe_equiv(prob: &MipProblem, mut postsolve: Vec<crate::presolve::PostsolveStep>, scaling: crate::presolve::scaling::Scaling, verbose: bool, submip: bool) -> Result<Presolved, (Vec<crate::presolve::PostsolveStep>, crate::presolve::scaling::Scaling)> {
    // 同値な 0-1 列の代入 (既定で使う。`ENOMOTO_MIP_NO_PROBE_EQUIV` で使わない): プロービングで見つけた同値な 0-1 列を等式 (`x_k - x_j = 0` / `x_k + x_j = 1`) として足し、
    // 前処理をもう一度かける (二項等式の消去が列を代入で消す)。かけ直しは `ENOMOTO_T_MIP_PROBE_EQUIV_ROUNDS` 回まで (既定 2)
    let equiv = PROBE_EQUIV.with(|e| std::mem::take(&mut *e.borrow_mut()));
    let depth = PROBE_EQUIV_DEPTH.with(|d| d.get());
    // 時間の上限・止めの合図 ([`presolve_stop`]) を過ぎていれば、代入せずに (前処理をかけ直さずに) 返す (代入しない問題も正しい)。
    // FJ のスレッドの FJ_P3 の前処理が期限の後にかけ直しを 1 回 (遅い機械で 0.5〜1 秒) 回していた (analysis/overrun_crypt_20261101.md)
    if !equiv.is_empty() && depth < tunable!("ENOMOTO_T_MIP_PROBE_EQUIV_ROUNDS", 2usize, usize) && !presolve_stop() {
        // `ENOMOTO_MIP_EQUIV_EXTEND_FIRST` (試作、既定では使わない): 組の等式を足す前に、プロービングの含意を含む衝突グラフで
        // クリーク行の拡張 ([`extend_clique_rows`]) をかける (HiGHS の探りは延長 (持ち上げ) を先に入れてから同値の列を代入する)。
        // physiciansched6-2 では代入を先にすると、その後の延長が弱く (HiGHS の探りでも持ち上げ 1745 -> 140〜580 個)、
        // HiGHS の根のカットも 49324 に届かない。延長を先にすると HiGHS は 13 ラウンドで 49324 に着く (analysis/phys_equiv_20261025.md)
        let ext_first = env_str!("ENOMOTO_MIP_EQUIV_EXTEND_FIRST").is_some() && !submip && env_str!("ENOMOTO_MIP_NO_CLIQUE_EXTEND").is_none();
        let base = if ext_first { extend_clique_rows(prob.clone(), verbose) } else { prob.clone() };
        let MipProblem { col_lo, col_up, cost, offset, sense_sign, is_int, mut rows, mut row_lo, mut row_up, .. } = base;
        for &(j, k, neg) in &equiv {
            let (a, b) = (j.min(k), j.max(k));
            rows.push(vec![(a, 1.0), (b, if neg { 1.0 } else { -1.0 })]);
            let r = if neg { 1.0 } else { 0.0 };
            row_lo.push(r);
            row_up.push(r);
        }
        let q = MipProblem::from_rows(col_lo, col_up, cost, offset, sense_sign, is_int, rows, row_lo, row_up);
        // 診断用 (`ENOMOTO_MIP_DEBUG_DUMP_PROBE=接頭辞`): 組の等式を足した問題を `接頭辞_q番号.mps` に書く
        if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_DUMP_PROBE").filter(|_| !submip) {
            static CNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let f = format!("{f}_q{}.mps", CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
            dump_mps(&q, &f);
            eprintln!("MIP: probing: dumped the problem with {} equivalence rows to {f} (offset {})", equiv.len(), q.offset);
        }
        PROBE_EQUIV_DEPTH.with(|d| d.set(depth + 1));
        let r = presolve_mip(&q, verbose, submip, None);
        PROBE_EQUIV_DEPTH.with(|d| d.set(depth));
        match r {
            Some(Presolved::Infeasible) => return Ok(Presolved::Infeasible),
            Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) if sc2.d.iter().chain(&scaling.d).all(|&d| d == 1.0) => {
                postsolve.extend(post2);
                return Ok(Presolved::Reduced { prob: prob2, postsolve, scaling });
            }
            _ => {}
        }
    }
    Err((postsolve, scaling))
}

/// 締め付けでの前処理のかけ直しの形 (`ENOMOTO_MIP_PRR_TIGHT` の値)。既定は `o` (締め付けだけで固定の無い周にだけ使う。
/// analysis/rootbound_20261126 の 6 節)、`0` か `off` で使わない (従来の形)、`1` などそれ以外の値は固定のある周でも使う
fn prr_tight_mode() -> Option<&'static str> {
    match env_str!("ENOMOTO_MIP_PRR_TIGHT") {
        None => Some("o"),
        Some("0") | Some("off") => None,
        Some(v) => Some(v),
    }
}

/// プロービングの後の前処理のかけ直し (試作、`ENOMOTO_MIP_PROBE_REPRESOLVE` で使う。既定では使わない): プロービングが
/// 境界を締めた (固定した) とき、締めた境界で前処理をもう一度かける (`ENOMOTO_T_MIP_PROBE_REPRESOLVE_MAX` 回まで、既定 2)。
/// プロービングは連続列の境界も締める (drayage-25-23 では時刻の列の境界) が、その境界で冗長になった行・強められる係数は
/// 前処理をかけ直さないと残る。drayage-25-23 では 3212 行のまま根を解き、再スタートの前処理で初めて 3873 -> 1150 行になる
/// (HiGHS は前処理の中で 732 行まで減らす)。かけ直した前処理は新しい問題でプロービングをやり直す (含意はそちらで集め直す)。
/// 前処理は列の番号を保ち、整数最適化では列の尺度をかけない (d = 1) ので、後処理は内側の手順の後に外側の手順を並べればよい
fn represolve_after_probe(prob: MipProblem, mut postsolve: Vec<crate::presolve::PostsolveStep>, scaling: crate::presolve::scaling::Scaling, verbose: bool, submip: bool, depth: u32) -> Option<Presolved> {
    // 試作 `ENOMOTO_MIP_PRB_TABLE` の `b` ([`prb_table`]): かけ直しを `ENOMOTO_T_MIP_PRB_TABLE_ROUNDS` (8) 回まで、行の減り方によらず使う
    // `ENOMOTO_MIP_PRR_TIGHT` ([`prr_tight_mode`]、既定は `o`。analysis/cutgap_20261123 の 3 節): 固定が 0 本でも、プロービングが境界を締めたらかけ直し、
    // 締めた境界で前処理 (係数の強化) をかけて、またプロービングする、を繰り返す (SCIP は probing と linear の前処理を交互に回す。istanbul-no-cutoff
    // では 13 周で境界の変更 14294 本・係数の締め 47989)。回数の上限・行の減り方の判定は使わず、SCIP の前処理の打ち切り (`presolving/abortfac` 0.0008:
    // 1 周の変更が列の数のその割合より少なければやめる) と同じ形で、その周のプロービングの固定と締め付けが列の数の 0.0008 倍に届かなければやめる
    let mut tight = (prr_tight_mode().is_some() || enum_adapt_on()) && !submip && !PRESOLVE_IN_SUBMIP.with(|f| f.get());
    // `ENOMOTO_MIP_PRR_TIGHT=o` (analysis/rootbound_20261126 の 6 節): 締め付けだけで固定の無い周にだけ PRR_TIGHT の規則 (回数の上限・行の減り方の
    // 判定を使わない) を使い、固定のある周は既定の規則 (`PRR_MIN_FIXED`・`PRR_MIN_ROW_FRAC`・`PROBE_REPRESOLVE_MAX`) のままにする。
    // `=1` は固定のある周でも行の減り方の判定を外すので、misc07・dcmulti・nw04 (締め付け 0 本) の道筋まで変わっていた。
    // 2026-11-27 に `o` を既定にした (P で 1 スレッド 1.001・4 スレッド 1.013、istanbul-no-cutoff の根 107.7 -> 143.4)
    if tight && !enum_adapt_on() && prr_tight_mode() == Some("o") && !submip {
        let (gf, gt, _) = PROBE_GAIN.with(|f| f.get());
        let fixed = PROBE_FIXED.with(|f| f.get());
        if gt == 0 || gf > 0 || fixed >= tunable!("ENOMOTO_T_MIP_PRR_MIN_FIXED", 1usize, usize) {
            tight = false;
        }
    }
    // `ENOMOTO_MIP_ENUM_ADAPT` ([`enum_adapt_on`]): 締め付けでのかけ直し (と 0.0008 の打ち切り) を使うかを、直前の列挙の繰り返し全体の得と時間から
    // シグモイドで決める ([`enum_adapt_continue`])。使わないときは既定のかけ直しの規則 (固定の本数・行の減り方) に戻す
    if tight && enum_adapt_on() && prr_active(submip) {
        let (g, secs, el) = ENUM_LOOP_LAST.with(|c| c.get());
        tight = enum_adapt_continue(g, prob.n, secs, el, verbose, "presolve again");
    }
    let max = if tight { u32::MAX } else if prb_table('b') { tunable!("ENOMOTO_T_MIP_PRB_TABLE_ROUNDS", 8u32, u32) } else { tunable!("ENOMOTO_T_MIP_PROBE_REPRESOLVE_MAX", 2u32, u32) };
    if !prr_active(submip) || depth >= max {
        return Some(Presolved::Reduced { prob, postsolve, scaling });
    }
    if tight {
        let (gf, gt, _) = PROBE_GAIN.with(|f| f.get());
        if gf + gt == 0 || (!enum_adapt_on() && ((gf + gt) as f64) < 0.0008 * prob.n as f64) {
            if verbose {
                eprintln!("MIP: presolve again after probing: PRR_TIGHT: stopped (probing fixed {gf}, tightened {gt} of {} columns)", prob.n);
            }
            return Some(Presolved::Reduced { prob, postsolve, scaling });
        }
    }
    // プロービングが列を `ENOMOTO_T_MIP_PRR_MIN_FIXED` 本 (既定 1) より少なくしか固定しなければ (境界を締めただけなら)、かけ直さない。
    // drayage-25-23 (連続列の境界を 8 本締めるだけ) では、かけ直しが行を 2〜5% しか減らさず、下の判定で捨てるかけ直しに再スタートの
    // 度に 0.4 秒ほど使っていた
    let fixed = PROBE_FIXED.with(|f| f.get());
    if !tight && fixed < tunable!("ENOMOTO_T_MIP_PRR_MIN_FIXED", 1usize, usize) {
        if verbose {
            eprintln!("MIP: presolve again after probing: skipped (probing fixed {fixed} columns)");
        }
        return Some(Presolved::Reduced { prob, postsolve, scaling });
    }
    if verbose {
        eprintln!("MIP: presolve again after probing (round {})", depth + 1);
    }
    // かけ直しが行を `ENOMOTO_T_MIP_PRR_MIN_ROW_FRAC` (既定 0.1) の割合より減らさなければ、かけ直した結果を捨てて元の問題を使う
    // (かけ直さない既定と同じ問題になる。0 なら減り方を見ない)。drayage-25-23 はかけ直しが行を 752 -> 732 (2.7%)・列を 1 本しか
    // 減らさないのに、根の LP の解がずれて根の clique のサブ MIP が解を出さなくなり、根の被約費用の固定が再スタートの閾値に届かずに
    // 6 秒 -> 13〜60 秒になっていた。かけ直しが効く momentum1・traininstance6 は行を 38〜43% 減らす (analysis/prr_harm_20261106.md)。
    // 捨てるときは、かけ直しの中のプロービングが置いた同値な組を元に戻す
    let min_frac = if prb_table('b') || tight { 0.0 } else { tunable!("ENOMOTO_T_MIP_PRR_MIN_ROW_FRAC", 0.1, f64) };
    let saved_equiv = (min_frac > 0.0).then(|| (PROBE_EQUIV.with(|e| e.borrow().clone()), PROBE_EQUIV_FORCE.with(|f| f.get())));
    let precheck = min_frac > 0.0 && env_str!("ENOMOTO_MIP_PRR_PRECHECK").is_some_and(|v| v != "0");
    let prev_pc = PRR_PRECHECK.with(|c| c.replace(precheck.then_some((depth + 1, prob.m, min_frac))));
    let r = presolve_mip_depth(&prob, verbose, submip, None, depth + 1);
    PRR_PRECHECK.with(|c| c.set(prev_pc));
    if let (Some((eq, force)), Some(Presolved::Reduced { prob: prob2, .. })) = (&saved_equiv, &r) {
        if (prob.m as f64 - prob2.m as f64) < min_frac * prob.m as f64 {
            if verbose {
                eprintln!("MIP: presolve again after probing: rows {} -> {} (less than {:.0}% fewer), the result is not used", prob.m, prob2.m, 100.0 * min_frac);
            }
            PROBE_EQUIV.with(|e| *e.borrow_mut() = eq.clone());
            PROBE_EQUIV_FORCE.with(|f| f.set(*force));
            return Some(Presolved::Reduced { prob, postsolve, scaling });
        }
    }
    if !submip && !PRESOLVE_IN_SUBMIP.with(|f| f.get()) {
        PRR_APPLIED.with(|f| f.set(true));
    }
    match r {
        Some(Presolved::Infeasible) => Some(Presolved::Infeasible),
        Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) => {
            debug_assert!(sc2.d.iter().chain(&scaling.d).all(|&d| d == 1.0));
            postsolve.extend(post2);
            Some(Presolved::Reduced { prob: prob2, postsolve, scaling })
        }
        // 何も減らない: プロービングの結果 (含意・締めた境界) を持つ元の問題を使う
        None => Some(Presolved::Reduced { prob, postsolve, scaling }),
    }
}

/// LP による固定 (既定で使う。`ENOMOTO_MIP_NO_LP_PROBE` で使わない。前処理の最後、含意を集めるプロービングの前): LP 緩和を解き、LP 解で分数値の整数列
/// x_j = v ごとに、上の子 (x_j >= ceil v) と下の子 (x_j <= floor v) の LP を反復上限つきで解く。片方が実行不能なら逆側に
/// 固定し (根の強分枝で子が実行不能になったときの固定と同じ。どの整数解でも成り立つ)、両方なら問題は実行不能。固定したら
/// LP を解き直し、新しく分数値になった列で繰り返す (`ENOMOTO_T_MIP_LP_PROBE_PASSES` 回まで)。手間の上限は LP の反復の合計
/// (`ENOMOTO_T_MIP_LP_PROBE_ITER_MULT` x 最初の LP の反復 + 5000) と時間 `secs`。最後の LP の基底は根の LP に渡す
/// ([`solver::ROOT_BASIS`])。
///
/// app1-1 では 0-1 列 1030 個のうち 925 個が「1 にすると LP が実行不能」で、LP 解の分数値の 19 個を固定するだけで LP 値が
/// −20.96 → −3.98 (目的関数は整数なので下界 −3 = 最適値) になる。HiGHS は経路集約・tableau のカットで根を −4 にし、こちらは
/// 根のカットのループで −7〜−8.6 にしかならず、木の根の強分枝で初めて −3.99 に届いていた。
/// 固定した列の数を返す。実行不能と分かったら `None`。
/// `ENOMOTO_MIP_TIGHTEN_VUBS` (試作): CBC の `CbcModel::tightenVubs` (CbcModel.cpp 15861-16130) にならい、VUB の行
/// (固定されていない列が連続列 1 本と 0-1 列 1 本だけの行、`x <= M y` など) の連続列について、LP で `x` を最小化・最大化して
/// 境界を締める。LP の解の値は LP の緩和の実行可能領域の端なので、整数解の集合は変わらない。CBC と同じく、締めるのは
/// 改善が 1e-4 を超えるときだけで、`1e-8 (1 + |境界|)` だけ緩めて置く。候補は `ENOMOTO_T_MIP_TIGHTEN_VUBS_MAX` 本
/// (既定 200、行の多い列から)、時間は `ENOMOTO_T_MIP_TIGHTEN_VUBS_TIME` 秒 (既定 1 秒) まで。締めた境界の本数を返す
/// (最初の LP が実行不能なら `None`)。
fn tighten_vubs(prob: &mut MipProblem, verbose: bool) -> Option<usize> {
    use lp::{LpStatus, SolveLimits};
    let t0 = std::time::Instant::now();
    let n = prob.n;
    let free = |j: usize| prob.col_up[j] - prob.col_lo[j] > 1e-8;
    let binary = |j: usize| prob.is_int[j] && prob.col_lo[j] == 0.0 && prob.col_up[j] == 1.0;
    let mut cand = vec![false; n];
    for r in &prob.rows {
        let (mut nb, mut nc, mut cont) = (0usize, 0usize, usize::MAX);
        for &(j, _) in r {
            if !free(j) {
                continue;
            }
            if binary(j) {
                nb += 1;
            } else {
                nc += 1;
                cont = j;
            }
        }
        if nc == 1 && nb == 1 && !prob.is_int[cont] {
            cand[cont] = true;
        }
    }
    let mut list: Vec<usize> = (0..n).filter(|&j| cand[j]).collect();
    if list.is_empty() {
        return Some(0);
    }
    list.sort_by_key(|&j| std::cmp::Reverse(prob.cols[j].len()));
    list.truncate(tunable!("ENOMOTO_T_MIP_TIGHTEN_VUBS_MAX", 200usize, usize));
    let deadline = t0 + std::time::Duration::from_secs_f64(tunable!("ENOMOTO_T_MIP_TIGHTEN_VUBS_TIME", 1.0, f64));
    let mut c = vec![0.0; n];
    let mut lp = crate::simplex::mip_lp::TwoStageLp::new(&prob.col_lo, &prob.col_up, &c, &prob.rows, &prob.row_lo, &prob.row_up);
    match lp.solve(&SolveLimits { deadline: Some(deadline), ..Default::default() }) {
        LpStatus::Optimal => {}
        LpStatus::Infeasible => return None,
        _ => return Some(0),
    }
    let (mut k, mut solved) = (0usize, 0usize);
    'cols: for &j in &list {
        for up in [false, true] {
            if std::time::Instant::now() > deadline {
                break 'cols;
            }
            c[j] = if up { -1.0 } else { 1.0 };
            lp.set_costs(&c);
            let st = lp.solve(&SolveLimits { deadline: Some(deadline), ..Default::default() });
            c[j] = 0.0;
            solved += 1;
            if st != LpStatus::Optimal {
                if st == LpStatus::Infeasible {
                    return None;
                }
                continue;
            }
            let v = lp.col_value(j);
            let (lo, hi) = (prob.col_lo[j], prob.col_up[j]);
            if up && v < hi - 1e-4 {
                let mut nv = hi.min(v + 1e-8 * (hi.abs() + 1.0));
                if nv - lo < 1e-7 {
                    nv = lo;
                }
                prob.col_up[j] = nv;
                lp.set_col_bounds(j, lo, nv);
                k += 1;
            } else if !up && v > lo + 1e-4 {
                let mut nv = lo.max(v - 1e-8 * (lo.abs() + 1.0));
                if hi - nv < 1e-7 {
                    nv = hi;
                }
                prob.col_lo[j] = nv;
                lp.set_col_bounds(j, nv, hi);
                k += 1;
            }
        }
    }
    if verbose {
        eprintln!("MIP: presolve: tightened {k} bounds of {} VUB columns by {solved} LPs (TIGHTEN_VUBS, {:.2}s)", list.len(), t0.elapsed().as_secs_f64());
    }
    Some(k)
}

/// `try_fix` が偽なら LP を解いて基底を渡すだけ (列の固定は試さない。今の呼び出しはどれも真。偽で呼んでいた試作
/// `ENOMOTO_MIP_LP_PROBE_AFTER_EXT` は 2026-11-18 に削除)。
/// LP のプロービングの LP が実行不能と出たとき、その証明が際どく信用しないか ([`solver::lp_farkas_marginal`]、`ENOMOTO_MIP_NO_LPINF_CHECK` で止める)。
/// 信用しないときは数える (根の LP・LP のプロービングの欄)
fn lp_inf_untrusted<L: lp_api::MipLp>(lp: &L, n: usize) -> bool {
    let u = solver::lpinf_check_on() && solver::lp_farkas_marginal(lp, n);
    if u {
        solver::lpinf_count(3);
    }
    u
}

/// `ENOMOTO_MIP_VBCLQ` の `e` (既定、`=0` で使わない。analysis/scipgate_20261120): 前処理の LP による固定の前に、暫定解の値 z を上限 (`cost · x <= z - offset`、
/// 暫定解そのものは残る) にして [`vbclq::pseudoobj`] で 0-1 列を固定し、定義域の伝播の結果も列の境界に書く。暫定解の出どころ:
/// - VB01 ([`heur_scip::vbounds01_on_problem`]、SCIP-VB の試作の Solver の外の版)。
/// - 診断 `ENOMOTO_MIP_VBCLQ_PRE_CUTOFF=<値>` (元の問題の目的の値。解は持たない、上限だけ。VB01 の解より良ければこちら)。
/// 解があれば [`VBCLQ_E_SOL`] に置き、前処理の後で開始解にする。境界を変えた列の数を返す。解が目的の最小値に届いて最適と証明できたら
/// [`FEAS_FJ_SOL`] に置いて `usize::MAX` を返す (問題は変えない)
fn presolve_vbclq_e(prob: &mut MipProblem, verbose: bool, secs: f64) -> usize {
    let t0 = std::time::Instant::now();
    // VB01 (SCIP-VB の Solver の外の版、[`heur_scip::vbounds01_on_problem`]) の解。サブ MIP 1 本の時間の上限は `secs` (前処理の
    // LP による固定の時間の上限と同じ安全網。サブ MIP は 500 ノードまで)
    let vb = heur_scip::vbounds01_on_problem(prob, verbose, secs, PRESOLVE_SEED.with(|f| f.get()).unwrap_or_else(solver::default_seed));
    let given = env_str!("ENOMOTO_MIP_VBCLQ_PRE_CUTOFF").and_then(|s| s.parse::<f64>().ok()).map(|v| prob.sense_sign * v);
    let (z, src) = match (&vb, given) {
        (Some((zv, _)), Some(zg)) if zg < *zv => (zg, "given cutoff"),
        (Some((zv, _)), _) => (*zv, "VB01 solution"),
        (None, Some(zg)) => (zg, "given cutoff"),
        (None, None) => {
            if verbose {
                eprintln!("MIP: VBCLQ e: before the LP probing: no solution ({:.2}s)", t0.elapsed().as_secs_f64());
            }
            return 0;
        }
    };
    let vb_x = vb.map(|(_, x)| x);
    let mut dom = domain::Domain::new(prob);
    if !prob.conflict_edges.is_empty() {
        dom.add_implications(prob.n, &prob.conflict_edges);
    }
    if !dom.propagate(prob) {
        return 0;
    }
    let binary: Vec<bool> = (0..prob.n).map(|j| prob.is_int[j] && prob.col_lo[j] >= 0.0 && prob.col_up[j] <= 1.0).collect();
    let g = clique::CliqueGraph::build_with(prob.n, &prob.rows, &prob.row_lo, &prob.row_up, &binary, &prob.col_lo, &prob.col_up, &prob.conflict_edges);
    let o = vbclq::pseudoobj(prob, &mut dom, &g, z - prob.offset);
    vbclq::count_file(&o, "e");
    // 目的の最小値 (pseudoobj の最初の余裕 = 上限 − 目的の最小値) が解の値に届いていれば (|余裕| が許容誤差の内)、その解が最適と
    // 証明済み: 前処理の残りと木を飛ばす ([`FEAS_FJ_SOL`] の道。decomp2 は VB01 の解 −160 で余裕 0)
    let tol = 1e-9 * (z - prob.offset).abs().max(1.0);
    if let Some(x) = vb_x.as_ref().filter(|_| o.slack0.abs() <= tol && src == "VB01 solution") {
        if verbose {
            eprintln!("MIP: VBCLQ e: the VB01 solution {z:.10e} reaches the pseudo-objective bound (slack {:.3e}); optimal ({:.2}s)", o.slack0, t0.elapsed().as_secs_f64());
        }
        vbclq::count_file(&vbclq::PseudoObjOut { infeas: true, ..o }, "eopt");
        FEAS_FJ_SOL.with(|s| *s.borrow_mut() = Some(x.clone()));
        VBCLQ_E_OPT.with(|f| f.set(true));
        return usize::MAX;
    }
    if let Some(x) = vb_x {
        VBCLQ_E_SOL.with(|s| *s.borrow_mut() = Some(x));
    }
    // 上限より良い解がない (暫定解が最適) ときも、固定は書かない (暫定解の値の解は残るはずなので、数値の揺れとみなす)
    let mut changed = 0usize;
    if !o.infeas && o.fixed > 0 {
        for j in 0..prob.n {
            if dom.lo[j] > prob.col_lo[j] || dom.up[j] < prob.col_up[j] {
                changed += 1;
                prob.col_lo[j] = prob.col_lo[j].max(dom.lo[j]);
                prob.col_up[j] = prob.col_up[j].min(dom.up[j]);
            }
        }
    }
    if verbose {
        eprintln!(
            "MIP: VBCLQ e: before the LP probing, bound {:.10e} ({src}): {} objective columns substituted, {} fixed in {} passes{}, slack {:.6e}, {changed} columns changed ({:.2}s)",
            z,
            o.nsub,
            o.fixed,
            o.passes,
            if o.infeas { ", no solution better than the bound (not applied)" } else { "" },
            o.slack0,
            t0.elapsed().as_secs_f64()
        );
    }
    changed
}

fn lp_probe(prob: &mut MipProblem, verbose: bool, secs: f64, try_fix: bool) -> Option<usize> {
    let _pt = pretime::guard("lpprobe");
    use lp::{LpStatus, SolveLimits};
    let t0 = std::time::Instant::now();
    let deadline = Some(t0 + std::time::Duration::from_secs_f64(secs.max(0.0)));
    let mut lp = crate::simplex::mip_lp::TwoStageLp::new(&prob.col_lo, &prob.col_up, &prob.cost, &prob.rows, &prob.row_lo, &prob.row_up);
    // `ENOMOTO_MIP_LP_PROBE_FINISH` (試作): 最初の LP の期限を `secs` の `ENOMOTO_T_MIP_LP_PROBE_FINISH_MULT` 倍 (既定 4) にし、
    // 渡す基底に行 (論理変数) の状態も入れる (根の LP の行の数が同じときだけ使う)。期限 `secs` を過ぎて解けたときは列の
    // 固定は試さない (下の `budget_left`) が、基底は根の LP に渡す。hypothyroid-k1 (4 スレッド・60 秒) では最初の LP が
    // 3.0〜3.1 秒で期限 3 秒に当たって解いた分を捨て、根で同じ LP を基底なしで解き直していた (3067 反復、5〜7 秒)。
    // 列の状態だけの基底を渡しても、行の状態がないので別の基底になり、根の LP は同じ 3067 反復かかった
    // 値が `late` なら、最初の LP が期限 `secs` を過ぎて解けたときだけ (期限の中で解けたときは行の状態・DSE 重みを渡さない)。
    // neos-960392 は最初の LP が 2.1〜2.3 秒 (期限 3 秒) で解け、重みを引き継いだ根の LP が別の頂点に着くと根の RENS が
    // 最適値を出さず時間切れになる (analysis/cand9bisect_20261024.md)
    // 0.1.7 から既定で使わない (束 cand1114 の prune1112)。`1`・`late` で使う
    let lpf = env_str!("ENOMOTO_MIP_LP_PROBE_FINISH").filter(|v| *v != "0" && *v != "off");
    let mut finish = lpf.is_some();
    let finish_late = lpf == Some("late");
    // `ENOMOTO_MIP_PRESOLVE_WORK` (試作): 時間 `secs` の代わりに LP の手間 (反復の数 x (行 + 列 + 非零)) の上限で打ち切る。
    // `ENOMOTO_T_MIP_PRESOLVE_WORK_LP` は `secs` (60 秒で 3 秒) に、`_LP_GIVEUP` は 0.1 秒 (反復 x (行 + 列)) に、`_LP_BASIS` は 1 秒に当たる値。
    // `secs` はここでは安全網 (呼び出し側で大きくしてある)
    let work = presolve_work();
    let size = (prob.m + prob.n + prob.nnz).max(1) as f64;
    let iters_of = |w: f64| (w / size).max(1.0) as u64;
    let w_lp = tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_LP", 1.25e9, f64);
    let lp_iter_cap = if work { iters_of(w_lp) } else { u64::MAX };
    // `ENOMOTO_MIP_LP_PROBE_FIRST_FULL` (試作、既定では使わない): 最初の LP の期限を `secs` の `ENOMOTO_T_MIP_LP_PROBE_FIRST_MULT` 倍
    // (既定 4 倍) に延ばす (固定を試す手間の上限は `secs` のまま)。最初の LP が `secs` に間に合わないと基底を根に渡せず、根で同じ LP を
    // もう一度解く (時間の損) うえ、着く頂点が変わる。neos-960392 (60 秒で `secs` = 3.0 秒、最初の LP は 2.8〜3.0 秒) では、
    // 間に合った回は根の RENS が 0 ノードで -238 (最適値) を出して 29 秒で終わり、間に合わなかった回は別の頂点からの RENS が
    // -237 で時間切れになり、60 秒で証明できなかった
    let first_deadline = if finish {
        Some(t0 + std::time::Duration::from_secs_f64((tunable!("ENOMOTO_T_MIP_LP_PROBE_FINISH_MULT", 4.0, f64) * secs).max(0.0)))
    } else if env_str!("ENOMOTO_MIP_LP_PROBE_FIRST_FULL").is_some() {
        Some(t0 + std::time::Duration::from_secs_f64((tunable!("ENOMOTO_T_MIP_LP_PROBE_FIRST_MULT", 4.0, f64) * secs).max(0.0)))
    } else {
        deadline
    };
    let first_deadline = if work { deadline } else { first_deadline };
    let first_iters = if !work {
        u64::MAX
    } else if finish || env_str!("ENOMOTO_MIP_LP_PROBE_FIRST_FULL").is_some() {
        // 最初の LP の上限 (`ENOMOTO_T_MIP_PRESOLVE_WORK_LP_FIRST`、60 秒の期限 12 秒に当たる値。uccase12 は 5.6e9 で 7.6 秒)
        iters_of(tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_LP_FIRST", 1.0e10, f64))
    } else {
        lp_iter_cap
    };
    // `ENOMOTO_MIP_ROOTLP_SCALE` (既定で使う。`=0` で使わない): 最初の LP も尺度を調整した写しで解く ([`TwoStageLp::request_scale_once`])
    if env_def!("ENOMOTO_MIP_ROOTLP_SCALE", "1").is_some() {
        lp.request_scale_once();
    }
    let tk_first0 = crate::ticks::total();
    match lp.solve(&SolveLimits { deadline: first_deadline, iteration_limit: first_iters, ..Default::default() }) {
        LpStatus::Optimal => {}
        // 際どい実行不能 ([`solver::lp_farkas_marginal`]) は信用しない: 固定をせずに終える (根の LP で解き直す)
        LpStatus::Infeasible if lp_inf_untrusted(&lp, prob.n) => return Some(0),
        LpStatus::Infeasible => return None,
        st => {
            if verbose {
                eprintln!("MIP: LP probing: the first LP stopped: {st:?} ({:.2}s, limit {secs:.2}s{})", t0.elapsed().as_secs_f64(), if finish { " x mult" } else { "" });
            }
            if !solver::in_submip_thread() {
                solver::ROOTCAP_PROBE_SECS.store(t0.elapsed().as_secs_f64().to_bits(), std::sync::atomic::Ordering::Relaxed);
            }
            // 試作 `ENOMOTO_MIP_ROOTCAP` の `p` ([`solver::rootcap`]): 止まった LP の基底 (行の状態・DSE 重みつき) を捨てずに根の LP に渡す
            // (根の LP はここから続きを解く。行が変わっていれば根の側で行の状態を捨てる)
            // `ENOMOTO_MIP_ROOTCAP` の `q`: 渡す前に、同じ LP (尺度・DSE 重み・摂動をそのまま) を `ENOMOTO_T_MIP_ROOTCAP_P_MULT` (2) x 止まるまでの秒だけ
            // 続けて解く (最適まで解ければ、根の LP はその基底からすぐ終わる。止まればやはり渡す)
            let mut st = st;
            if solver::rootcap('q') && !solver::in_submip_thread() && matches!(st, LpStatus::TimeLimit | LpStatus::IterationLimit) {
                let t1 = std::time::Instant::now();
                let d = t1 + std::time::Duration::from_secs_f64(tunable!("ENOMOTO_T_MIP_ROOTCAP_P_MULT", 2.0, f64) * t0.elapsed().as_secs_f64());
                let it0 = lp.total_iterations();
                st = lp.solve(&SolveLimits { deadline: Some(d), ..Default::default() });
                if verbose {
                    eprintln!("MIP: LP probing: ROOTCAP q: continued the first LP: {st:?} (+{} iters, {:.2}s)", lp.total_iterations() - it0, t1.elapsed().as_secs_f64());
                }
                if st == LpStatus::Infeasible && !lp_inf_untrusted(&lp, prob.n) {
                    return None;
                }
            }
            if (solver::rootcap('p') || solver::rootcap('q')) && !solver::in_submip_thread() && matches!(st, LpStatus::TimeLimit | LpStatus::IterationLimit | LpStatus::Optimal) {
                let b = lp.basis();
                let with_rows = b.row.len() == prob.m;
                if with_rows {
                    solver::ROOT_BASIS_ROWS.with(|s| *s.borrow_mut() = Some(rows_fingerprint(&prob.rows)));
                    let w = lp.dse_snapshot();
                    solver::ROOT_DSE.with(|s| *s.borrow_mut() = w);
                }
                let b = if with_rows { b } else { lp::Basis { col: b.col, row: Vec::new() } };
                if verbose {
                    eprintln!("MIP: LP probing: ROOTCAP p: passing the stopped LP's basis to the root LP (rows {with_rows}, {} iters)", lp.total_iterations());
                }
                solver::ROOT_BASIS.with(|s| *s.borrow_mut() = Some(b));
            }
            return Some(0);
        }
    }
    let z0 = lp.objective() + prob.offset;
    let root_secs = t0.elapsed().as_secs_f64();
    if !solver::in_submip_thread() {
        solver::ROOTCAP_PROBE_SECS.store(root_secs.to_bits(), std::sync::atomic::Ordering::Relaxed);
    }
    pretime::add("lpprobe_first", root_secs);
    let root_iters = lp.total_iterations();
    // 試作 `ENOMOTO_MIP_LPPROBE_TICKS` (既定にせず閉じた、2026-11-21、analysis/scipsym_20261120 の README の「結論」。[`lpprobe_ticks`]): 最初の LP の決定的な手間 (`crate::ticks`) と、子の LP の手間を数え始める点
    let root_ticks = crate::ticks::total() - tk_first0;
    let tk_probe0 = crate::ticks::total();
    let tk_extra = std::cell::Cell::new(0.0f64);
    let lpp_ticks = lpprobe_ticks() && work;
    // `=w` では反復のあきらめの代わりに使う (反復の数えは止める)
    let lpp_w = env_str!("ENOMOTO_MIP_LPPROBE_TICKS") == Some("w");
    let probe_ticks = || crate::ticks::total() - tk_probe0 + tk_extra.get();
    if finish_late && (if work { root_iters < lp_iter_cap } else { root_secs < secs }) {
        finish = false;
    }
    let iter_budget = (tunable!("ENOMOTO_T_MIP_LP_PROBE_ITER_MULT", 2.0, f64) * root_iters as f64) as u64 + 5000;
    let per_lp = (root_iters / 4).clamp(200, 2000);
    let passes = if try_fix { tunable!("ENOMOTO_T_MIP_LP_PROBE_PASSES", 3usize, usize) } else { 0 };
    // 最初の `ENOMOTO_T_MIP_LP_PROBE_GIVEUP` 列で 1 つも固定できなければやめる (固定が出る問題では最初からすぐ出る:
    // app1-1 は最初の 24 列で 21 個。40 問で固定が出るのは 5 問で、出ない問題に 1 問平均 0.5 秒、最大 3 秒かかっていた)
    let giveup = tunable!("ENOMOTO_T_MIP_LP_PROBE_GIVEUP", 20usize, usize);
    // `ENOMOTO_MIP_PRESOLVE_WORK`: 0.1 秒のあきらめ (下の `giveup_secs`) の代わりに、列の数 `ENOMOTO_T_MIP_PRESOLVE_WORK_LP_GIVEUP_COLS`
    // (既定 8) と反復 (`giveup_iters`) で打ち切る。反復 1 回の重さは問題で 7 倍以上違い (drayage-100-23 は 94 マイクロ秒、cvs16r128-89 は
    // 380 マイクロ秒)、反復だけでは drayage-100-23 (固定 38 列) と roll3000・cvs16r128-89 (既定は 8 列・1 列であきらめる) を両立できない
    let giveup = if presolve_work() { giveup.min(tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_LP_GIVEUP_COLS", 8usize, usize)) } else { giveup };
    // 固定が 1 つもないまま、最初の LP の `ENOMOTO_T_MIP_LP_PROBE_GIVEUP_TIME_MULT` 倍 (+ 0.1 秒) の時間を使ってもやめる
    // (子の LP が重い問題: cvs16r128-89 は 5 列で 3 秒、根の LP は 0.9 秒)
    let giveup_secs = tunable!("ENOMOTO_T_MIP_LP_PROBE_GIVEUP_TIME_MULT", 1.0, f64) * root_secs + 0.1;
    let giveup_secs = if work { f64::INFINITY } else { giveup_secs };
    // 試作 `ENOMOTO_MIP_GAP2_LPPGIVE=g` (既定では使わない。analysis/gap2_rootcost_20261109.md): `ENOMOTO_MIP_PRESOLVE_WORK` (既定) の
    // 反復によるあきらめに加え、固定が 1 つもないまま最初の LP の後の子の LP に g x 最初の LP の秒 (壁時計) を使ってもやめる。
    // 反復の手間の見積もり (反復 x (行 + 列)) は密な行の問題で軽く出る: cod105 (1024 行、1 行 56 非零) は最初の LP が 11.4 秒、
    // 子の LP 4 本 (2 列) に 18.6 秒使い、固定 0 のまま時間の上限 (時間の上限の 5% = 30 秒) まで回っていた
    let giveup_secs = match env_str!("ENOMOTO_MIP_GAP2_LPPGIVE").and_then(|v| v.parse::<f64>().ok()).filter(|&g| g > 0.0) {
        Some(g) => giveup_secs.min((1.0 + g) * root_secs),
        None => giveup_secs,
    };
    // あきらめの手間は反復 x (行 + 列) で数える (非零を入れると、反復の安い列の長い問題 (neos-860300) で 1 列目であきらめる)
    let giveup_iters = if work {
        (tunable!("ENOMOTO_T_MIP_LP_PROBE_GIVEUP_TIME_MULT", 1.0, f64) * root_iters as f64) as u64 + (tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_LP_GIVEUP", 2.0e7, f64) / (prob.m + prob.n).max(1) as f64) as u64
    } else {
        u64::MAX
    };
    // `=w`: 今の反復のあきらめ (`giveup_iters` の子の LP の分) を、最初の LP の 1 反復の手間で ticks に直したもの (新しい定数なし、
    // 1 反復の重さを最初の LP の平均でなく子の LP の実際の手間で数える)。ほかの値: 最初の LP の手間の倍率 (`..._GIVEUP_TIME_MULT`、既定 1) まで
    let giveup_ticks = if env_str!("ENOMOTO_MIP_LPPROBE_TICKS") == Some("w") {
        giveup_iters.saturating_sub(root_iters) as f64 * root_ticks / root_iters.max(1) as f64
    } else {
        tunable!("ENOMOTO_T_MIP_LP_PROBE_GIVEUP_TIME_MULT", 1.0, f64) * root_ticks
    };
    // `ENOMOTO_MIP_PREPAR` の `l` (試作、[`prepar_has`]): 1 巡の中の列の試しは、どれも巡の始めの状態 (`saved`) から解くので互いに
    // 独立。候補の窓を塊に分け、塊ごとの LP の写し (巡の始めに作る) で順に試し、決まった順にまとめる。写しで使った反復は
    // `it_extra` に足して、逐次と同じく手間の上限・あきらめに数える。窓・塊の大きさはスレッドの数によらない
    let par_threads = PREPAR_THREADS.with(|t| t.get());
    let lpar = try_fix && prepar_has('l') && par_threads >= 1 && !solver::in_submip_thread();
    let lpool = lpar.then(|| rayon::ThreadPoolBuilder::new().num_threads(par_threads).build().ok()).flatten();
    let lpar_blocks = tunable!("ENOMOTO_T_MIP_PREPAR_BLOCKS", 4usize, usize).max(1);
    let it_extra = std::cell::Cell::new(0u64);
    let tot_it = |lp: &crate::simplex::mip_lp::TwoStageLp| lp.total_iterations() + it_extra.get();
    // `ENOMOTO_MIP_SUBMIP_HALT`: 止める合図が立ったら残りの試しをやめる (各 LP は合図で止まるが、ループは予算まで回っていた)
    let budget_left = |lp: &crate::simplex::mip_lp::TwoStageLp| tot_it(lp) - root_iters < iter_budget && tot_it(lp) < lp_iter_cap && t0.elapsed().as_secs_f64() < secs && !crate::cancel::halt_requested();
    let (mut n_lps, mut n_fixed, mut n_tried) = (0usize, 0usize, 0usize);
    // 窓の数と、試した列の反復の和と数 (窓の候補の数の見積もりに使う)
    let (mut n_lwin, mut it_lc, mut n_lc) = (0usize, 0u64, 0usize);
    let mut tried: std::collections::HashSet<(usize, i64)> = std::collections::HashSet::new();
    let mut ok = true;
    // 試作 `ENOMOTO_MIP_LPPROBE_FIX` の `d`: 子の LP の間は厳密な DSE 重みを使い回す (窓の写しは塊ごとに立てる)
    let lpp_dse = lpprobe_fix('d');
    if lpp_dse {
        crate::simplex::slope_intercept_dual::lpprobe_dse_scope(true);
    }
    // 診断 `ENOMOTO_MIP_LPPROBE_DIAG` + `ENOMOTO_MIP_XPROF`: 最初の LP までの区間計時を出して消す (子の LP の分を別に見る)
    let lpp_xprof = lpprobe_diag() && env_str!("ENOMOTO_MIP_XPROF").is_some();
    if lpp_xprof {
        for (l, ns) in crate::simplex::slope_intercept_dual::xprof_take() {
            eprintln!("LPPDIAG xprof first {l:12} {:10.1} ms", ns as f64 / 1e6);
        }
        crate::simplex::slope_intercept_dual::xprof_enable(false);
        crate::simplex::slope_intercept_dual::xprof_enable(true);
    }
    if lpprobe_diag() {
        eprintln!("LPPDIAG first: rows {} cols {} nnz {} iters {root_iters} secs {root_secs:.3} per_lp {per_lp} iter_budget {iter_budget} giveup {giveup} giveup_iters {giveup_iters} work {work} par {} threads {par_threads}", prob.m, prob.n, prob.nnz, lpool.is_some());
    }
    for pass in 0..passes {
        let x = lp.col_values();
        let mut cands: Vec<(usize, f64)> = (0..prob.n)
            .filter(|&j| prob.is_int[j] && prob.col_lo[j] < prob.col_up[j] && (x[j] - x[j].round()).abs() > domain::FEASTOL && !tried.contains(&(j, x[j].floor() as i64)))
            .map(|j| (j, x[j]))
            .collect();
        // 分数部分が 1/2 に近い順
        cands.sort_by(|a, b| {
            let fa = a.1 - a.1.floor();
            let fb = b.1 - b.1.floor();
            (fb * (1.0 - fb)).total_cmp(&(fa * (1.0 - fa)))
        });
        if cands.is_empty() || !budget_left(&lp) {
            break;
        }
        let saved = lp.save_state();
        // (列, 上限を締めるか, 値)
        let mut fixes: Vec<(usize, bool, f64)> = Vec::new();
        // `ENOMOTO_MIP_PREPAR` の `l`: 塊ごとの LP の写し (この巡の始めの状態) と、窓の結果 (上の子が切れるか, 下の子が切れるか, 反復)
        let mut wk: Vec<crate::simplex::mip_lp::TwoStageLp> = Vec::new();
        let (mut win, mut win_start): (Vec<(bool, bool, u64, f64)>, usize) = (Vec::new(), 0);
        // 塊の大きさ: 最初の窓はあきらめの列の数 (`giveup`) を塊に分けた大きさ、窓ごとに倍 (`ENOMOTO_T_MIP_PREPAR_LP_BSZ_MAX` まで)
        let mut lbsz = (giveup / lpar_blocks).max(1);
        for (ci, &(j, v)) in cands.iter().enumerate() {
            if !budget_left(&lp) || (n_fixed + fixes.len() == 0 && (n_tried >= giveup || t0.elapsed().as_secs_f64() > giveup_secs || (tot_it(&lp) > giveup_iters && !(lpp_w && lpp_ticks)) || (lpp_ticks && probe_ticks() > giveup_ticks))) {
                if lpp_ticks && n_fixed + fixes.len() == 0 && probe_ticks() > giveup_ticks && budget_left(&lp) && !(n_tried >= giveup || t0.elapsed().as_secs_f64() > giveup_secs || tot_it(&lp) > giveup_iters) && verbose {
                    eprintln!("MIP: LP probing: LPPROBE_TICKS gave up after {n_tried} columns (child LP ticks {:.3e} > {:.3e} = mult x first LP)", probe_ticks(), giveup_ticks);
                }
                break;
            }
            tried.insert((j, v.floor() as i64));
            n_tried += 1;
            let (lo, up) = (prob.col_lo[j], prob.col_up[j]);
            let mut cut = [false; 2];
            // `ENOMOTO_MIP_PREPAR` の `l`: 窓の候補の数は、残りの反復の予算 (まだ固定がなければあきらめの反復まで) を 1 列あたりの
            // 反復 (まだなければ 1 列の上限 2 x `per_lp`) で割った数と、まだ固定がなければあきらめまでの列の数までにする。
            // 2 未満なら並べずにこの列を逐次に試す (予算で打ち切る巡で、使われない試しを並べない。数えた量だけで決まる)
            let in_win = ci < win_start + win.len();
            let mut wlen = 0usize;
            if lpool.is_some() && !in_win {
                if n_lwin > 0 {
                    lbsz = (lbsz * 2).min(tunable!("ENOMOTO_T_MIP_PREPAR_LP_BSZ_MAX", 16usize, usize).max(1));
                }
                wlen = lbsz * lpar_blocks;
                let per = if n_lc > 0 { (it_lc as f64 / n_lc as f64).max(1.0) } else { 2.0 * per_lp as f64 };
                let mut left = iter_budget.saturating_sub(tot_it(&lp) - root_iters).min(lp_iter_cap.saturating_sub(tot_it(&lp)));
                if n_fixed + fixes.len() == 0 {
                    if !(lpp_w && lpp_ticks) {
                        left = left.min(giveup_iters.saturating_sub(tot_it(&lp)));
                    }
                    wlen = wlen.min(giveup.saturating_sub(n_tried - 1));
                    // 試作 `ENOMOTO_MIP_LPPROBE_TICKS`: 窓の候補も、残りの手間 (最初の LP の手間の倍率 - 子の LP の手間) を 1 列の手間の見積り
                    // (1 列の反復 x 最初の LP の 1 反復の手間) で割った数までにする (窓を解いてからあきらめても、窓の分の時間は使ってしまうため)
                    if lpp_ticks {
                        let per_tk = per * root_ticks / root_iters.max(1) as f64;
                        if per_tk > 0.0 {
                            wlen = wlen.min(((giveup_ticks - probe_ticks()).max(0.0) / per_tk).ceil() as usize);
                        }
                    }
                }
                wlen = wlen.min((left as f64 / per).ceil() as usize);
            }
            if let Some(pool) = lpool.as_ref().filter(|_| in_win || wlen >= 2) {
                if !in_win {
                    let end = (ci + wlen).min(cands.len());
                    let nb = (end - ci).min(lpar_blocks);
                    while wk.len() < nb {
                        wk.push(lp.clone());
                    }
                    let rem = if work { lp_iter_cap.saturating_sub(tot_it(&lp)).max(1) } else { u64::MAX };
                    // `ENOMOTO_MIP_LPPROBE_FIX` の `c`: 窓の始めの効き目で子の上限を縮める
                    let per_lp = lpp_cap(per_lp, n_tried - 1, n_fixed + fixes.len());
                    let blocks: Vec<&[(usize, f64)]> = cands[ci..end].chunks((end - ci).div_ceil(nb).max(1)).collect();
                    let (col_lo, col_up, saved_r) = (&prob.col_lo, &prob.col_up, &saved);
                    let n_cols = prob.n;
                    let res: Vec<Vec<(bool, bool, u64, f64)>> = pool.install(|| {
                        use rayon::prelude::*;
                        wk.par_iter_mut()
                            .zip(blocks.par_iter())
                            .map(|(w, bl)| {
                                if lpp_dse {
                                    crate::simplex::slope_intercept_dual::lpprobe_dse_scope(true);
                                }
                                let r: Vec<(bool, bool, u64, f64)> = bl.iter()
                                    .map(|&(j, v)| {
                                        let it0 = w.total_iterations();
                                        let tk0 = crate::ticks::total();
                                        let mut c = [false; 2];
                                        for (side, is_up) in [(0usize, true), (1usize, false)] {
                                            if is_up {
                                                w.set_col_bounds(j, v.ceil(), col_up[j]);
                                            } else {
                                                w.set_col_bounds(j, col_lo[j], v.floor());
                                            }
                                            let (td, itd) = (std::time::Instant::now(), w.total_iterations());
                                            let st = w.solve(&SolveLimits { iteration_limit: per_lp.min(rem), deadline, ..Default::default() });
                                            c[side] = st == LpStatus::Infeasible && !lp_inf_untrusted(&*w, n_cols);
                                            if lpprobe_diag() {
                                                let zc = if st == LpStatus::Optimal || st == LpStatus::IterationLimit { w.objective() } else { f64::NAN };
                                                eprintln!("LPPDIAG child: col {j} v {v:.4} {} {st:?} iters {} secs {:.4} obj {zc:.6e} (window)", if is_up { "up" } else { "down" }, w.total_iterations() - itd, td.elapsed().as_secs_f64());
                                            }
                                            w.restore_state(saved_r);
                                        }
                                        (c[0], c[1], w.total_iterations() - it0, crate::ticks::total() - tk0)
                                    })
                                    .collect();
                                if lpp_dse {
                                    let st = crate::simplex::slope_intercept_dual::lpprobe_dse_stats_take();
                                    crate::simplex::slope_intercept_dual::lpprobe_dse_scope(false);
                                    if lpprobe_diag() {
                                        eprintln!("LPPDIAG dse reuse {} built {} mismatch {} (window block)", st[0], st[1], st[2]);
                                    }
                                }
                                r
                            })
                            .collect()
                    });
                    win = res.into_iter().flatten().collect();
                    win_start = ci;
                    n_lwin += 1;
                }
                let (c0, c1, it, tk) = win[ci - win_start];
                it_extra.set(it_extra.get() + it);
                tk_extra.set(tk_extra.get() + tk);
                it_lc += it;
                n_lc += 1;
                cut = [c0, c1];
                n_lps += 2;
            } else {
                let it0 = lp.total_iterations();
                for (side, is_up) in [(0usize, true), (1usize, false)] {
                    if is_up {
                        lp.set_col_bounds(j, v.ceil(), up);
                    } else {
                        lp.set_col_bounds(j, lo, v.floor());
                    }
                    // `ENOMOTO_MIP_PRESOLVE_WORK`: 反復の上限 `lp_iter_cap` を越えない (時間の期限で途中で止まる分を手間で決める)
                    // `ENOMOTO_MIP_LPPROBE_FIX` の `c`: この列の前までの効き目で子の上限を縮める
                    let per_lp = lpp_cap(per_lp, n_tried - 1, n_fixed + fixes.len());
                    let lim = if work { per_lp.min(lp_iter_cap.saturating_sub(tot_it(&lp)).max(1)) } else { per_lp };
                    let (td, itd) = (std::time::Instant::now(), lp.total_iterations());
                    let st = lp.solve(&SolveLimits { iteration_limit: lim, deadline, ..Default::default() });
                    cut[side] = st == LpStatus::Infeasible && !lp_inf_untrusted(&lp, prob.n);
                    if lpprobe_diag() {
                        let zc = if st == LpStatus::Optimal || st == LpStatus::IterationLimit { lp.objective() } else { f64::NAN };
                        eprintln!("LPPDIAG child: col {j} v {v:.4} {} {st:?} iters {} secs {:.4} obj {zc:.6e} (seq, lim {lim})", if is_up { "up" } else { "down" }, lp.total_iterations() - itd, td.elapsed().as_secs_f64());
                    }
                    n_lps += 1;
                    lp.restore_state(&saved);
                }
                it_lc += lp.total_iterations() - it0;
                n_lc += 1;
            }
            match (cut[0], cut[1]) {
                (true, true) => {
                    ok = false;
                    break;
                }
                // 上の子が実行不能なら上限を floor に、下の子なら下限を ceil に
                (true, false) => fixes.push((j, true, v.floor())),
                (false, true) => fixes.push((j, false, v.ceil())),
                _ => {}
            }
        }
        if verbose {
            eprintln!(
                "MIP: LP probing pass {}: {} candidates, {} fixings{} ({} LPs, {} iters, {:.2}s)",
                pass + 1,
                cands.len(),
                fixes.len(),
                if ok { "" } else { ", both sides infeasible" },
                n_lps,
                tot_it(&lp) - root_iters,
                t0.elapsed().as_secs_f64()
            );
        }
        if !ok || fixes.is_empty() {
            break;
        }
        for &(j, upper, val) in &fixes {
            if upper {
                prob.col_up[j] = prob.col_up[j].min(val);
            } else {
                prob.col_lo[j] = prob.col_lo[j].max(val);
            }
            lp.set_col_bounds(j, prob.col_lo[j], prob.col_up[j]);
        }
        n_fixed += fixes.len();
        // 固定した後の解き直しの反復の上限 (`ENOMOTO_MIP_PRESOLVE_WORK` のとき): 残りの手間と最初の LP の反復の大きい方
        let lim = if work { lp_iter_cap.saturating_sub(tot_it(&lp)).max(root_iters) } else { u64::MAX };
        match lp.solve(&SolveLimits { iteration_limit: lim, deadline, ..Default::default() }) {
            LpStatus::Optimal => {}
            LpStatus::Infeasible if !lp_inf_untrusted(&lp, prob.n) => {
                ok = false;
                break;
            }
            _ => break,
        }
    }
    if lpp_dse {
        let st = crate::simplex::slope_intercept_dual::lpprobe_dse_stats_take();
        crate::simplex::slope_intercept_dual::lpprobe_dse_scope(false);
        if lpprobe_diag() {
            eprintln!("LPPDIAG dse reuse {} built {} mismatch {} (main)", st[0], st[1], st[2]);
        }
    }
    if lpp_xprof {
        crate::simplex::slope_intercept_dual::xprof("lpp_tail");
        for (l, ns) in crate::simplex::slope_intercept_dual::xprof_take() {
            eprintln!("LPPDIAG xprof children {l:12} {:10.1} ms", ns as f64 / 1e6);
        }
        crate::simplex::slope_intercept_dual::xprof_enable(false);
        crate::simplex::slope_intercept_dual::xprof_enable(true);
    }
    if verbose && n_tried > 0 {
        eprintln!(
            "MIP: LP probing: tried {} columns, fixed {}, LP {:.10e} -> {:.10e} ({} LPs, {} iters, {:.2}s{})",
            n_tried,
            n_fixed,
            z0,
            lp.objective() + prob.offset,
            n_lps,
            tot_it(&lp) - root_iters,
            t0.elapsed().as_secs_f64(),
            if lpool.is_some() { format!(", PREPAR {par_threads} threads, {n_lwin} windows") } else { String::new() }
        );
    }
    if !ok {
        return None;
    }
    // 固定したときだけ最後の LP の基底を根の LP に渡す (固定がなければ根の LP は前処理の LP と同じ問題で、基底を渡すと
    // 着く頂点が変わり、カット・探索の経路が変わるだけ: qiu は固定 0 で 40 問の 2 回とも遅くなった)。ただし最初の LP に
    // `ENOMOTO_T_MIP_LP_PROBE_BASIS_SECS` 秒 (既定 1 秒) 以上かかったら、根で解き直す手間を省くために渡す
    // 試作 `ENOMOTO_MIP_HO4_CUT0` 用: 前処理の LP による固定の最初の LP の反復 (根の LP がこの基底から数反復で終わると、
    // 根のカットのループの「根の LP の反復」が LP の重さを表さなくなるので)
    HO4_PROBE_ITERS.with(|c| c.set(root_iters));
    // 試作 `ENOMOTO_MIP_SUBCAP` の `w` 用: 前処理の LP による固定の最初の LP の秒 (根の LP がこの基底から温め直しで終わると、
    // 根の LP の秒が LP の重さを表さなくなるので。supportcase42 は 28.7 秒 → 根の LP 0.05 秒)
    if !solver::in_submip_thread() {
        solver::SUBCAP_PROBE_SECS.store(root_secs.to_bits(), std::sync::atomic::Ordering::Relaxed);
    }
    let slow_root = if work { root_iters as f64 * size >= tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_LP_BASIS", 1.5e8, f64) } else { root_secs >= tunable!("ENOMOTO_T_MIP_LP_PROBE_BASIS_SECS", 1.0, f64) };
    // 試作 `ENOMOTO_MIP_P2BASIS=<秒>` (既定では使わない、analysis/par4slow_20261119/README.md): 手間の見積りで「遅い」としても、実際の
    // 最初の LP が <秒> 未満なら渡さない (渡すのは根で解き直す手間を省くためなので、速い LP では省ける分が小さく、頂点が変わるだけになる。
    // ns1208400 は 1 スレッド 1782 反復・4 スレッド 1952 反復で閾値 1.5e8 をまたぎ、0.36 秒の LP の基底が渡る 4 スレッドだけ木が大きい)
    let slow_root = slow_root && env_str!("ENOMOTO_MIP_P2BASIS").and_then(|v| v.parse::<f64>().ok()).is_none_or(|t| root_secs >= t);
    if verbose {
        eprintln!(
            "MIP: LP probing: root LP {root_secs:.2}s ({root_iters} iters, size {size}), tried {n_tried}, fixed {n_fixed}, basis passed {}, total {} iters {:.2}s, ticks first {:.3e} children {:.3e}",
            n_fixed > 0 || slow_root,
            tot_it(&lp),
            t0.elapsed().as_secs_f64(),
            root_ticks,
            probe_ticks()
        );
    }
    let passed = n_fixed > 0 || slow_root;
    // 診断 `ENOMOTO_MIP_PROBE_BASIS_DIAG`・試作 `ENOMOTO_MIP_PROBE_BASIS_GUARD` (既定では何もしない、analysis/probe_basis_20261106.md):
    // 最後の LP の基底と行の指紋・大きさ・最初の LP の手間を根の LP に渡す (根で列だけの基底の良し悪しを見る)
    if solver::probe_basis_diag_on() || env_def!("ENOMOTO_MIP_PROBE_BASIS_GUARD", "g").is_some() {
        let row_hashes = (0..prob.m).map(|i| solver::row_hash(&prob.rows[i], prob.row_lo[i], prob.row_up[i])).collect();
        let info = solver::ProbeBasisInfo { basis: lp.basis(), row_hashes, m: prob.m, nnz: prob.rows.iter().map(|r| r.len()).sum(), iters: root_iters, secs: root_secs, passed, n_fixed, rows_x: None };
        let want_rows = env_str!("ENOMOTO_MIP_PROBE_BASIS_DIAG").is_some_and(|v| v.contains(['f', 'g', 'h'])) || solver::probe_basis_guard_has('g') || solver::probe_basis_guard_has('h');
        let info = if want_rows && lp.basis().row.len() == prob.m {
            let b = lp.basis();
            solver::ProbeBasisInfo { rows_x: Some((prob.rows.iter().cloned().zip(b.row.iter().copied()).collect(), lp.col_values())), ..info }
        } else {
            info
        };
        solver::PROBE_BASIS_INFO.with(|s| *s.borrow_mut() = Some(info));
    }
    if passed || solver::probe_basis_guard_has('p') {
        let b = lp.basis();
        // 行の状態は `ENOMOTO_MIP_LP_PROBE_FINISH` か `ENOMOTO_MIP_LP_PROBE_ROW_BASIS` のときだけ渡す (既定は列の状態だけ)。行の指紋
        // ([`rows_fingerprint`]) も渡し、根の LP の始めの行が同じときだけ使う ([`solver::ROOT_BASIS_ROWS`])
        let with_rows = (finish || env_def!("ENOMOTO_MIP_LP_PROBE_ROW_BASIS", "1").is_some()) && b.row.len() == prob.m;
        if with_rows {
            solver::ROOT_BASIS_ROWS.with(|s| *s.borrow_mut() = Some(rows_fingerprint(&prob.rows)));
        }
        // `ENOMOTO_MIP_ROOT_BASIS_FIX` (試作): 行の中身と状態・解も渡す (根の LP の行が変わっていたら中身で照らし合わせて基底を作る)
        // 文字 `s` (再スタートの後の根だけ) なら置かない (root_probe は今まで通り)
        // 試作 `ENOMOTO_MIP_ROOT_BASIS_FIX_BIG` ([`solver::root_basis_fix_big`]): 非零の多い問題だけ同じく置く
        if ((solver::root_basis_fix_on() && !solver::root_basis_fix_has('s')) || solver::probe_basis_guard_has('f') || solver::root_basis_fix_big(&prob)) && b.row.len() == prob.m {
            let rows: Vec<(Vec<(usize, f64)>, lp::VarStatus)> = prob.rows.iter().cloned().zip(b.row.iter().copied()).collect();
            solver::ROOT_BASIS_SAVED.with(|s| *s.borrow_mut() = Some((rows, lp.col_values())));
        }
        let b = if with_rows { b } else { lp::Basis { col: b.col, row: Vec::new() } };
        solver::ROOT_BASIS.with(|s| *s.borrow_mut() = Some(b));
        if finish && with_rows {
            let w = lp.dse_snapshot();
            solver::ROOT_DSE.with(|s| *s.borrow_mut() = w);
        }
    }
    Some(n_fixed)
}

/// `ENOMOTO_MIP_PRESOLVE_WORK` (既定 1、`0` で外す。計測 21 で既定化): 前処理の中の打ち切り (LP による固定・プロービング・対称性の検出) を
/// 壁時計・CPU 時間ではなく手間 (LP の反復の数・伝播で走査した行の長さ・色の細分化で読んだ非零の数) の上限で決める。
/// 前処理の結果が時間の上限・機械の速さ・負荷・回で変わらないようにする。時間の上限は安全網として残す
/// ([`presolve_work_safe`] 倍)。
pub(crate) fn presolve_work() -> bool {
    env_def!("ENOMOTO_MIP_PRESOLVE_WORK", "1").is_some()
}

/// [`presolve_work`] のとき、元の時間の上限を何倍の安全網にするか (`ENOMOTO_T_MIP_PRESOLVE_WORK_SAFE`、既定 10 倍)。
pub(crate) fn presolve_work_safe() -> f64 {
    tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_SAFE", 10.0, f64)
}

/// プロービング (SCIP・HiGHS の前処理と同じ): 0-1 列 `x_j` を 0 にして伝播した結果と 1 にして伝播した結果を比べる。
///
/// - 両方矛盾すれば問題は実行不能 (`Some(false)` を返す)。
/// - 片方だけ矛盾すれば、もう片方の値に固定する。
/// - どちらも矛盾しなければ、各列の境界を両方の結果の弱い方 (下限は小さい方、上限は大きい方) まで締める
///   (どちらの値でも成り立つので、大域的に成り立つ)。
///
/// 列の値は変えない (境界を締めるだけ) ので後処理は要らない。手間は伝播で走査した行の長さの合計
/// (非零数の `ENOMOTO_T_MIP_PROBE_WORK` 倍、既定 20 倍) と時間 (`ENOMOTO_T_MIP_PROBE_TIME` 秒、既定 0.5 秒。固定・締め付けが
/// 出ていれば `ENOMOTO_T_MIP_PROBE_TIME_FOUND` 秒、既定 0.5 秒。長い行の多い問題 (nw04・air03) は 1 回が重く何も得られない) で打ち切る。
/// 締めた境界があれば `Some(true)`、何もなければ `None`。
/// [`probe`] の投機的な試しの結果 (`ENOMOTO_MIP_PREPAR` の `p`): x_j = 0 と x_j = 1 の伝播が成り立つか、変わった境界、
/// 数えた手間 (行の走査, 活動の更新, 境界の変更)
struct ProbeSpec {
    ok0: bool,
    ch0: Vec<(usize, bool, f64)>,
    ok1: bool,
    ch1: Vec<(usize, bool, f64)>,
    w: (u64, u64, u64),
    /// 両側が成り立つときの集計 (写しの定義域で求める。窓を作った時点から定義域が変わっていなければ主の定義域と同じ値)
    derived: Option<ProbeDerived>,
}

/// [`ProbeSpec`] の集計: 衝突の組・含意による境界・同値な組・両側で締まる境界 (列の番号の順)
struct ProbeDerived {
    edges: Vec<(u32, u32)>,
    implied: Vec<(u32, bool, u32, bool, f64)>,
    equiv: Vec<(usize, usize, bool)>,
    tight: Vec<(usize, bool, f64)>,
}

/// [`probe_spec_block`] が集計に使う設定 ([`probe`] の同名の値)
struct ProbeSpecCtx<'a> {
    bin0: &'a [bool],
    genint_on: bool,
    want_impl: bool,
    want_implied: bool,
    want_equiv: bool,
}

/// x_j の両側の変更 `ch0`・`ch1` (どちらの側も伝播が成り立ったとき) から、両側の弱い方 (下限は小さい方、上限は大きい方) が
/// 今の境界 `lo`・`up` より締まる列の境界 (列, 上限か, 値) を列の番号の順に返す ([`probe_derive`] の締め付けの部品。
/// 根の打ち切り値つきプロービング (`ENOMOTO_MIP_ROOT_OBJ_PROBE_HULL`) でも使う)。同じ列・側が何度も変わったら最後 (最も締まった) の値を使う
pub(super) fn probe_hull_bounds(j: usize, ch0: &[(usize, bool, f64)], ch1: &[(usize, bool, f64)], lo: &[f64], up: &[f64]) -> Vec<(usize, bool, f64)> {
    let mut first: std::collections::BTreeMap<(usize, bool), f64> = std::collections::BTreeMap::new();
    for &(k, upper, v) in ch0 {
        first.insert((k, upper), v);
    }
    let mut last1: std::collections::HashMap<(usize, bool), f64> = std::collections::HashMap::new();
    for &(k, upper, v) in ch1 {
        last1.insert((k, upper), v);
    }
    let mut tight = Vec::new();
    for (&(k, upper), &v0) in &first {
        if k == j {
            continue;
        }
        let Some(&v1) = last1.get(&(k, upper)) else { continue };
        if upper {
            let u = v0.max(v1);
            if u < up[k] - 1e-9 * (1.0 + u.abs()) {
                tight.push((k, true, u));
            }
        } else {
            let l = v0.min(v1);
            if l > lo[k] + 1e-9 * (1.0 + l.abs()) {
                tight.push((k, false, l));
            }
        }
    }
    tight
}

/// x_j の両側の変更 `ch0`・`ch1` から [`ProbeDerived`] を作る ([`probe`] の (true, true) の枝と同じ規則。ハッシュ表の順の代わりに
/// 列の番号の順に並べる)
fn probe_derive(j: usize, ch0: &[(usize, bool, f64)], ch1: &[(usize, bool, f64)], lo: &[f64], up: &[f64], cx: &ProbeSpecCtx) -> ProbeDerived {
    let mut edges = Vec::new();
    let mut implied = Vec::new();
    if cx.want_impl {
        for (v, ch) in [(1u32, ch1), (0u32, ch0)].into_iter().rev() {
            let lj = 2 * j as u32 + if v == 1 { 0 } else { 1 };
            for &(k, upper, val) in ch {
                if k == j || !cx.bin0[k] {
                    continue;
                }
                if upper && val <= 0.5 {
                    edges.push((lj, 2 * k as u32));
                } else if !upper && val >= 0.5 {
                    edges.push((lj, 2 * k as u32 + 1));
                }
            }
        }
        if cx.want_implied {
            for (vb, ch) in [(false, ch0), (true, ch1)] {
                let mut best: std::collections::BTreeMap<(usize, bool), f64> = std::collections::BTreeMap::new();
                for &(k, upper, val) in ch {
                    if k == j {
                        continue;
                    }
                    let e = best.entry((k, upper)).or_insert(val);
                    *e = if upper { e.min(val) } else { e.max(val) };
                }
                for ((k, upper), val) in best {
                    if cx.genint_on && cx.bin0[k] {
                        continue;
                    }
                    let (l, u) = (lo[k], up[k]);
                    if !l.is_finite() || !u.is_finite() {
                        continue;
                    }
                    let gain = if upper { u - val } else { val - l };
                    if gain > 1e-6 * (1.0 + val.abs()) {
                        implied.push((j as u32, vb, k as u32, upper, val));
                    }
                }
            }
        }
    }
    let mut first: std::collections::BTreeMap<(usize, bool), f64> = std::collections::BTreeMap::new();
    for &(k, upper, v) in ch0 {
        first.insert((k, upper), v);
    }
    let mut last1: std::collections::HashMap<(usize, bool), f64> = std::collections::HashMap::new();
    for &(k, upper, v) in ch1 {
        last1.insert((k, upper), v);
    }
    let mut equiv = Vec::new();
    if cx.want_equiv {
        for (&(k, upper), &v0) in &first {
            if k == j || !cx.bin0[k] || lo[k] == up[k] {
                continue;
            }
            let Some(&v1) = last1.get(&(k, !upper)) else { continue };
            if upper && v0 <= 0.5 && v1 >= 0.5 {
                equiv.push((j, k, false));
            } else if !upper && v0 >= 0.5 && v1 <= 0.5 {
                equiv.push((j, k, true));
            }
        }
    }
    let tight = probe_hull_bounds(j, ch0, ch1, lo, up);
    ProbeDerived { edges, implied, equiv, tight }
}

/// 塊の候補を、`base` の写しで順に試す (各候補の後に巻き戻す)。`base` で固定されている候補は `None`
fn probe_spec_block(prob: &MipProblem, base: &domain::Domain, js: &[usize], cx: &ProbeSpecCtx) -> Vec<Option<ProbeSpec>> {
    let tc = std::time::Instant::now();
    let mut d = base.clone();
    if env_str!("ENOMOTO_MIP_PREPAR_DEBUG").is_some() {
        eprintln!("PREPAR clone {:.4}s", tc.elapsed().as_secs_f64());
    }
    let fine = presolve_stop_fine();
    js.iter()
        .map(|&j| {
            // `ENOMOTO_MIP_PRESOLVE_STOP_FINE`: 止めの合図の後は試さない (`None` の候補は主が逐次に試すが、主も候補ごとに合図を見て抜ける)
            if d.lo[j] == d.up[j] || (fine && presolve_stop()) {
                return None;
            }
            let s = (d.debug_work(), domain::act_work(), domain::chg_count());
            let pos = d.stack_len();
            d.tighten_upper(prob, j, 0.0);
            let ok0 = d.propagate(prob);
            let ch0: Vec<_> = if ok0 { d.changes_since(pos).collect() } else { Vec::new() };
            d.backtrack_to(prob, pos);
            d.tighten_lower(prob, j, 1.0);
            let ok1 = d.propagate(prob);
            let ch1: Vec<_> = if ok1 { d.changes_since(pos).collect() } else { Vec::new() };
            d.backtrack_to(prob, pos);
            let e = (d.debug_work(), domain::act_work(), domain::chg_count());
            let derived = (ok0 && ok1).then(|| probe_derive(j, &ch0, &ch1, &d.lo, &d.up, cx));
            Some(ProbeSpec { ok0, ch0, ok1, ch1, w: (e.0 - s.0, e.1 - s.1, e.2 - s.2), derived })
        })
        .collect()
}

/// [`probe`] の同値な組の巡 (`ENOMOTO_MIP_PREPAR` の `e`): 塊の候補を `base` の写しで順に試し、候補ごとに見つけた組
/// (列の番号の順) と数えた手間 (行の走査) を返す。固定されている候補・どちらかの側が成り立たない候補は組なし
fn probe_equiv_block(prob: &MipProblem, base: &domain::Domain, bin0: &[bool], js: &[usize]) -> Vec<(Vec<(usize, usize, bool)>, u64, u64)> {
    let mut d = base.clone();
    let fine = presolve_stop_fine();
    js.iter()
        .map(|&j| {
            let mut pairs = Vec::new();
            if d.lo[j] == d.up[j] || (fine && presolve_stop()) {
                return (pairs, 0, 0);
            }
            let w0 = d.debug_work();
            let (a0, c0) = (domain::act_work(), domain::chg_count());
            let pos = d.stack_len();
            d.tighten_upper(prob, j, 0.0);
            let ok0 = d.propagate(prob);
            let mut ch0: Vec<(usize, bool, f64)> = if ok0 { d.changes_since(pos).collect() } else { Vec::new() };
            d.backtrack_to(prob, pos);
            d.tighten_lower(prob, j, 1.0);
            let ok1 = d.propagate(prob);
            let ch1: std::collections::HashMap<(usize, bool), f64> = if ok1 { d.changes_since(pos).map(|(k, u, v)| ((k, u), v)).collect() } else { Default::default() };
            d.backtrack_to(prob, pos);
            let w = d.debug_work() - w0;
            let pw = equiv2_pwork(w, domain::act_work() - a0, domain::chg_count() - c0);
            if !(ok0 && ok1) {
                return (pairs, w, pw);
            }
            // 逐次の巡と同じく、同じ (列, 側) の変更は最後の値を使う
            let mut last0: std::collections::HashMap<(usize, bool), f64> = std::collections::HashMap::new();
            for &(k, u, v) in &ch0 {
                last0.insert((k, u), v);
            }
            ch0.sort_by_key(|&(k, u, _)| (k, u));
            ch0.dedup_by_key(|t| (t.0, t.1));
            for &(k, upper, _) in &ch0 {
                let v0 = last0[&(k, upper)];
                if k == j || !bin0[k] || d.lo[k] == d.up[k] {
                    continue;
                }
                let Some(&v1) = ch1.get(&(k, !upper)) else { continue };
                if upper && v0 <= 0.5 && v1 >= 0.5 {
                    pairs.push((j, k, false));
                } else if !upper && v0 >= 0.5 && v1 <= 0.5 {
                    pairs.push((j, k, true));
                }
            }
            (pairs, w, pw)
        })
        .collect()
}

/// `ENOMOTO_MIP_EQUIV2_PWORK` (2026-11-19 から既定、`=0` で以前の形。analysis/clqprbg_20261119・analysis/equiv2pw_20261119): 同値な組の巡 (`PROBE_CLIQUE=equiv`) の手間を、本体の
/// プロービングの `pwork` と同じ重み (およそナノ秒: 2.5 x 行の走査 + 5 x 活動量の更新 + 68 x 境界の変更) で数える。巡は行の走査 (`debug_work`)
/// だけを数えていて、活動量の更新が重い問題 (mzzv42z: 本体で work 74 万に対し pwork 8.5 億) では手間の上限が効かず、時間の安全網 (5 秒) まで走っていた
fn equiv2_pwork(rows: u64, act: u64, chg: u64) -> u64 {
    let (ww, wa, wc) = (
        tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE_W_ROW", 2.5, f64),
        tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE_W_ACT", 5.0, f64),
        tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE_W_CHG", 68.0, f64),
    );
    (ww * rows as f64 + wa * act as f64 + wc * chg as f64) as u64
}

/// 診断 `ENOMOTO_MIP_DEBUG_INTCENSUS` (CLQPRB の続き、analysis/clqprb_20261112.md §7): 整数列の幅と暗黙の整数の本数を数え、
/// `INTCENSUS {json}` の 1 行を標準エラーに出す (求解は変えない)。`lo`・`up` はその時点の (大域的な) 境界、`base` は
/// クリーク表・衝突グラフを作った時点の境界 (根の後の時点で、後から [0,1] に締まった整数列が表に載っていないかを数える)
pub(crate) fn int_census(p: &MipProblem, lo: &[f64], up: &[f64], base: Option<(&[f64], &[f64])>, tag: &str) {
    let n = p.n;
    let free = |j: usize| lo[j] < up[j];
    let (mut b01, mut w1nz, mut w2, mut w3, mut wide) = (0usize, 0usize, 0usize, 0usize, 0usize);
    for j in 0..n {
        if !p.is_int[j] || !free(j) {
            continue;
        }
        let w = up[j] - lo[j];
        if w == 1.0 {
            if lo[j] == 0.0 { b01 += 1 } else { w1nz += 1 }
        } else if w == 2.0 {
            w2 += 1
        } else if w == 3.0 {
            w3 += 1
        } else {
            wide += 1
        }
    }
    let integral = |v: f64| (v - v.round()).abs() <= 1e-9 * v.abs().max(1.0);
    // 暗黙の整数 ([`MipProblem::implied_integers`] と同じ規則)。`dual`: 双対側の規則も使う、`fixc`: 固定された列を定数として右辺に移す
    let implied = |dual: bool, fixc: bool| -> Vec<bool> {
        let mut mark = p.is_int.clone();
        let fixed = |k: usize| fixc && lo[k] == up[k];
        loop {
            let mut changed = false;
            for j in 0..n {
                if mark[j] || !free(j) {
                    continue;
                }
                // 行 i の、j と固定列 (fixc のとき) を除く列がすべて印つきで係数が a の整数倍か。固定列の寄与 (定数) を返す
                let rest = |i: usize, a: f64| -> Option<f64> {
                    let mut c = 0.0;
                    for &(k, b) in &p.rows[i] {
                        if k == j {
                            continue;
                        }
                        if fixed(k) {
                            c += b * lo[k];
                        } else if !(mark[k] && integral(b / a)) {
                            return None;
                        }
                    }
                    Some(c)
                };
                let ok = p.cols[j].iter().any(|&(i, a)| p.row_lo[i] == p.row_up[i] && rest(i, a).is_some_and(|c| integral((p.row_lo[i] - c) / a)))
                    || (dual && p.cols[j].len() == 1 && p.cost[j] != 0.0 && {
                        let (i, a) = p.cols[j][0];
                        let side = if (p.cost[j] > 0.0) == (a > 0.0) { p.row_lo[i] } else { p.row_up[i] };
                        [lo[j], up[j]].iter().all(|&v| !v.is_finite() || integral(v)) && rest(i, a).is_some_and(|c| !side.is_finite() || integral((side - c) / a))
                    });
                if ok {
                    mark[j] = true;
                    changed = true;
                }
            }
            if !changed {
                return mark;
            }
        }
    };
    let cnt = |m: &[bool]| (0..n).filter(|&j| m[j] && !p.is_int[j] && free(j)).count();
    let (pr, prd, prf, prdf) = (implied(false, false), implied(true, false), implied(false, true), implied(true, true));
    let pr01 = (0..n).filter(|&j| prf[j] && !p.is_int[j] && free(j) && up[j] - lo[j] == 1.0).count();
    let cont_free = (0..n).filter(|&j| !p.is_int[j] && free(j)).count();
    // 連続列で実質 0-1: [0,1] の連続列が、固定でない列がちょうど 2 本の等式で、係数の絶対値が同じ 0-1 の整数列と結ばれている
    let bin = |k: usize| p.is_int[k] && lo[k] == 0.0 && up[k] == 1.0;
    let cont01 = (0..n)
        .filter(|&j| {
            !p.is_int[j] && lo[j] == 0.0 && up[j] == 1.0 && p.cols[j].iter().any(|&(i, a)| {
                if p.row_lo[i] != p.row_up[i] {
                    return false;
                }
                let others: Vec<(usize, f64)> = p.rows[i].iter().copied().filter(|&(k, _)| k != j && free(k)).collect();
                others.len() == 1 && bin(others[0].0) && (others[0].1.abs() - a.abs()).abs() <= 1e-9 * a.abs()
            })
        })
        .count();
    // 表を作った時点より後に [0,1] に締まった整数列 (表を作った時点では 0-1 でなかった)
    let late01 = base.map_or(0, |(bl, bu)| (0..n).filter(|&j| bin(j) && !(bl[j] == 0.0 && bu[j] == 1.0)).count());
    let late_w1 = base.map_or(0, |(bl, bu)| (0..n).filter(|&j| p.is_int[j] && free(j) && up[j] - lo[j] == 1.0 && lo[j] != 0.0 && bu[j] - bl[j] > 1.0).count());
    eprintln!(
        "INTCENSUS {{\"at\":\"{tag}\",\"n\":{n},\"m\":{},\"int_free\":{},\"bin01\":{b01},\"w1_lo_nz\":{w1nz},\"w2\":{w2},\"w3\":{w3},\"wide\":{wide},\"cont_free\":{cont_free},\"impl_primal\":{},\"impl_primal_dual\":{},\"impl_primal_fixc\":{},\"impl_primal_dual_fixc\":{},\"impl_primal_fixc_w1\":{pr01},\"cont01_linked\":{cont01},\"late01\":{late01},\"late_w1_lo_nz\":{late_w1}}}",
        p.m,
        (0..n).filter(|&j| p.is_int[j] && free(j)).count(),
        cnt(&pr),
        cnt(&prd),
        cnt(&prf),
        cnt(&prdf)
    );
}

/// 診断 `ENOMOTO_MIP_DEBUG_CLQACT` (CLQACT の数え上げ、analysis/clqprb_20261112.md §10): 前処理の後の問題と含意 (`conflict_edges`) で
/// 衝突グラフを作り、行ごと・向きごと (`a x <= b` の形) に 0-1 の項の文字を寄与の大きい順に貪欲にクリークに分け、クリークごとの寄与を
/// 「最大の 1 つ」にしたとき、(1) 最大の活動量が下がる行、(2) それで冗長になる行、(3) 係数の締め付け (0-1 列 x_k を 0 にしたとき
/// 行が冗長になる) が新しく効く係数、(4) 最小の活動量が上がる行、(5) それで境界が締まる列 (0-1 列の固定を含む) を数え、
/// `CLQACT {json}` の 1 行を出す (求解は変えない)。文字の数が `ENOMOTO_T_MIP_CLQACT_MAXLITS` (既定 1000) を超える行は見ない
pub(crate) fn clqact_census(prob: &MipProblem, tag: &str) {
    let t0 = std::time::Instant::now();
    let n = prob.n;
    let (lo, up) = (&prob.col_lo, &prob.col_up);
    let binary: Vec<bool> = (0..n).map(|j| prob.is_int[j] && lo[j] == 0.0 && up[j] == 1.0).collect();
    let g = clique::CliqueGraph::build_with(n, &prob.rows, &prob.row_lo, &prob.row_up, &binary, lo, up, &prob.conflict_edges);
    let maxlits = tunable!("ENOMOTO_T_MIP_CLQACT_MAXLITS", 1000usize, usize);
    // 文字と重み (正) の列を貪欲にクリークに分け、(クリークごとの最大の和, 文字ごとの「その文字を除いたときの和の減り」) を返す
    // `own`: この行 (の同じ向き) 自身が 2 文字の衝突を導く余裕 (重みの和がこれを超える組は行そのものから出る組なので使わない。
    // 使うと集合詰め込みの行が自分の辺で冗長に見える)
    let partition = |lits: &mut Vec<(f64, u32)>, own: f64| -> (f64, Vec<f64>) {
        lits.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut cls: Vec<Vec<usize>> = Vec::new();
        let mut of = vec![0usize; lits.len()];
        for (q, &(w, l)) in lits.iter().enumerate() {
            let mut put = None;
            for (c, cl) in cls.iter().enumerate().take(64) {
                if cl.iter().all(|&o| w + lits[o].0 <= own && g.adjacent(l, lits[o].1)) {
                    put = Some(c);
                    break;
                }
            }
            match put {
                Some(c) => {
                    cls[c].push(q);
                    of[q] = c;
                }
                None => {
                    of[q] = cls.len();
                    cls.push(vec![q]);
                }
            }
        }
        // 重みの大きい順に入れたので、クリークの先頭が最大、2 番目が次
        let total: f64 = cls.iter().map(|cl| lits[cl[0]].0).sum();
        let dec: Vec<f64> = (0..lits.len())
            .map(|q| {
                let cl = &cls[of[q]];
                if cl[0] == q { lits[q].0 - cl.get(1).map_or(0.0, |&o| lits[o].0) } else { 0.0 }
            })
            .collect();
        (total, dec)
    };
    let (mut rows_max, mut rows_red, mut coefs, mut rows_min, mut bnd_cont, mut bnd_int, mut fix_bin, mut skipped) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let mut seen_red = vec![false; prob.m];
    for (i, r) in prob.rows.iter().enumerate() {
        for (sign, b, b_other) in [(1.0, prob.row_up[i], -prob.row_lo[i]), (-1.0, -prob.row_lo[i], prob.row_up[i])] {
            if !b.is_finite() {
                continue;
            }
            let tol = 1e-9 * (1.0 + b.abs());
            // 最大の活動量: 0-1 の項は「寄与が増える文字」(a > 0 なら x、a < 0 なら 1 - x) と増分 |a|、ほかの列は端
            let (mut base_max, mut base_min, mut oth_span, mut fin) = (0.0, 0.0, 0.0, true);
            let (mut up_lits, mut dn_lits): (Vec<(f64, u32)>, Vec<(f64, u32)>) = (Vec::new(), Vec::new());
            let mut other: Vec<(usize, f64)> = Vec::new();
            for &(j, a0) in r {
                let a = sign * a0;
                if lo[j] == up[j] {
                    base_max += a * lo[j];
                    base_min += a * lo[j];
                } else if binary[j] {
                    if a > 0.0 {
                        up_lits.push((a, 2 * j as u32));
                        dn_lits.push((a, 2 * j as u32 + 1));
                        base_min += a;
                    } else {
                        up_lits.push((-a, 2 * j as u32 + 1));
                        dn_lits.push((-a, 2 * j as u32));
                        base_max += a;
                    }
                } else {
                    let (mx, mn) = if a > 0.0 { (a * up[j], a * lo[j]) } else { (a * lo[j], a * up[j]) };
                    if !mx.is_finite() || !mn.is_finite() {
                        fin = false;
                    }
                    base_max += mx;
                    base_min += mn;
                    oth_span += mx - mn;
                    other.push((j, a));
                }
            }
            if !fin || up_lits.len() < 2 {
                continue;
            }
            if up_lits.len() > maxlits {
                skipped += 1;
                continue;
            }
            let old_up: f64 = up_lits.iter().map(|t| t.0).sum();
            // この向きで行自身が導く組: 文字を全部偽・ほかの列を最小の端にした活動量 (base_max - oth_span) からの余裕を重みの和が超える組
            let own_up = b - (base_max - oth_span) + tol;
            let (new_up, dec_up) = partition(&mut up_lits, own_up);
            let (max_old, max_new) = (base_max + old_up, base_max + new_up);
            if max_new < max_old - tol {
                rows_max += 1;
                if max_new <= b + tol && max_old > b + tol && !seen_red[i] {
                    rows_red += 1;
                    seen_red[i] = true;
                }
                // 係数の締め付け: 文字 l を偽にしたときの最大の活動量が b を下回れば、l の係数を減らせる (新しく効くものだけ)
                if max_new > b + tol {
                    for (q, &(w, _)) in up_lits.iter().enumerate() {
                        let old_wo = max_old - w;
                        let new_wo = max_new - dec_up[q];
                        if new_wo < b - tol && old_wo >= b - tol {
                            coefs += 1;
                        }
                    }
                }
            }
            // 最小の活動量: 0-1 の項は「寄与が減る文字」と減り |a| (base_min は全部が増える側の値)
            let old_dn: f64 = dn_lits.iter().map(|t| t.0).sum();
            // 減る側の文字の組は、逆の向き (-a x <= b_other) から出うる。その向きの最小の活動量は -(この向きの最大の活動量)
            let own_dn = if b_other.is_finite() { b_other + base_min + oth_span + tol } else { f64::INFINITY };
            let (new_dn, dec_dn) = partition(&mut dn_lits, own_dn);
            let (min_old, min_new) = (base_min - old_dn, base_min - new_dn);
            if min_new > min_old + tol {
                rows_min += 1;
                for &(j, a) in &other {
                    // a x_j <= b - (min - a の端)
                    let cmin = if a > 0.0 { a * lo[j] } else { a * up[j] };
                    let (bo, bn) = ((b - (min_old - cmin)) / a, (b - (min_new - cmin)) / a);
                    let improved = if a > 0.0 { bn < bo - 1e-9 * (1.0 + bo.abs()) && bn < up[j] - 1e-9 * (1.0 + up[j].abs()) } else { bn > bo + 1e-9 * (1.0 + bo.abs()) && bn > lo[j] + 1e-9 * (1.0 + lo[j].abs()) };
                    if improved {
                        if prob.is_int[j] { bnd_int += 1 } else { bnd_cont += 1 }
                    }
                }
                // 0-1 列: 文字 l を真 (寄与 +w) にすると最小の活動量が b を超えるなら固定できる (新しく効くものだけ)
                for (q, &(w, _)) in dn_lits.iter().enumerate() {
                    // dn_lits の文字 l の補を真にすると寄与が w 増える: 最小の活動量は (l を除いた最小) + w
                    let old_with = min_old + w;
                    let new_with = min_new + dec_dn[q];
                    if new_with > b + tol && old_with <= b + tol {
                        fix_bin += 1;
                    }
                }
            }
        }
    }
    eprintln!(
        "CLQACT {{\"at\":\"{tag}\",\"m\":{},\"edges\":{},\"rows_max_down\":{rows_max},\"rows_redundant\":{rows_red},\"coefs_tighten\":{coefs},\"rows_min_up\":{rows_min},\"bounds_cont\":{bnd_cont},\"bounds_int\":{bnd_int},\"fix_bin\":{fix_bin},\"rows_skipped\":{skipped},\"secs\":{:.2}}}",
        prob.m,
        g.num_edges(),
        t0.elapsed().as_secs_f64()
    );
}

/// 診断 `ENOMOTO_MIP_DEBUG_IMPLCLASS` (analysis/scipslow_20261119 の 14 節、求解は変えない): プロービングの終わりの含意 (衝突の組、
/// 重複と固定の列を除いた後) を、(a) 行だけの衝突グラフ ([`clique::CliqueGraph::build_with_stop`]、今の境界) に既にある組、
/// (b) 1 本の行の最小活動量で出る 2 文字の衝突だが、行の上限 (文字数 `MAX_ROW_LITS`・辺の数 `MAX_ROW_EDGES`) で落ちた組、
/// (c) 最小活動量が無限 (境界が無限の列を含む) の行に 2 列がそろって入っている組 (1 本の行の伝播では 0-1 列に届かないので、
/// 実際の出どころは (d) と同じく連鎖)、(d) 1 本の行と x_i = v だけの伝播では出ない組に分け、`IMPLCLASS {json}` の 1 行を出す。
/// 1 本の行の伝播で 0-1 列が偽になるのは、その行の最小活動量に x_i = v の増分を足すと x_j = w の増分が収まらないときだけなので、
/// (b) の判定は重みの和と余裕の比べと同じ。行ごとの文字数の分布 (片側ごと、2 文字以上の行) と上限に当たる行も出す。
fn implclass_diag(prob: &MipProblem, edges: &[(u32, u32)], e_init: usize, lo: &[f64], up: &[f64]) {
    let t0 = std::time::Instant::now();
    let n = prob.n;
    let binary: Vec<bool> = (0..n).map(|j| prob.is_int[j] && lo[j] >= 0.0 && up[j] <= 1.0 && lo[j] < up[j]).collect();
    let g = clique::CliqueGraph::build(n, &prob.rows, &prob.row_lo, &prob.row_up, &binary, lo, up);
    // 片側ごと (行 i, 符号 s): 最小活動量が有限か、余裕、文字数、組の数 (2 本の指)
    let m = prob.rows.len();
    let mut side: Vec<[(bool, f64, usize, u64); 2]> = vec![[(false, 0.0, 0, 0); 2]; m];
    let mut hist = [0u64; 8]; // 文字数: 2-10, 11-100, 101-1000, 1001-2000, 2001-1e4, 1e4-1e5, >1e5, (未使用)
    let (mut cap_lits, mut cap_edges, mut inf_rows) = (0usize, 0usize, 0usize);
    let mut cap_lens: Vec<usize> = Vec::new();
    for (i, r) in prob.rows.iter().enumerate() {
        for (si, (sign, b)) in [(1.0, prob.row_up[i]), (-1.0, -prob.row_lo[i])].into_iter().enumerate() {
            if !b.is_finite() {
                continue;
            }
            let (mut minact, mut ok) = (0.0f64, true);
            let mut w: Vec<f64> = Vec::new();
            for &(j, a0) in r {
                let a = sign * a0;
                let mm = if a > 0.0 { a * lo[j] } else { a * up[j] };
                if !mm.is_finite() {
                    ok = false;
                }
                minact += mm;
                if binary[j] && a != 0.0 {
                    w.push(a.abs());
                }
            }
            if !ok {
                if w.len() >= 2 {
                    inf_rows += 1;
                }
                side[i][si] = (false, f64::NAN, w.len(), 0);
                continue;
            }
            let slack = b - minact;
            let tol = 1e-9 * (1.0 + b.abs());
            w.sort_by(|x, y| y.total_cmp(x));
            let mut pairs = 0u64;
            let mut hi = w.len();
            for u in 0..w.len() {
                while hi > u + 1 && w[u] + w[hi - 1] <= slack + tol {
                    hi -= 1;
                }
                if hi <= u + 1 {
                    break;
                }
                pairs += (hi - u - 1) as u64;
            }
            side[i][si] = (true, slack, w.len(), pairs);
            if w.len() >= 2 && pairs > 0 {
                let l = w.len();
                hist[if l <= 10 { 0 } else if l <= 100 { 1 } else if l <= 1000 { 2 } else if l <= 2000 { 3 } else if l <= 10_000 { 4 } else if l <= 100_000 { 5 } else { 6 }] += 1;
                if l > MAX_ROW_LITS_DIAG {
                    cap_lits += 1;
                    cap_lens.push(l);
                } else if pairs > MAX_ROW_EDGES_DIAG {
                    cap_edges += 1;
                    cap_lens.push(l);
                }
            }
        }
    }
    // 列ごとの (行, 係数)、行の番号順
    let mut col_rows: Vec<Vec<(u32, f64)>> = vec![Vec::new(); n];
    for (i, r) in prob.rows.iter().enumerate() {
        for &(j, a) in r {
            col_rows[j].push((i as u32, a));
        }
    }
    let mut pairs: Vec<(u32, u32, bool)> = edges
        .iter()
        .enumerate()
        .filter(|&(_, &(a, b))| a / 2 != b / 2 && binary[(a / 2) as usize] && binary[(b / 2) as usize])
        .map(|(k, &(a, b))| (a.min(b), a.max(b), k >= e_init))
        .collect();
    pairs.sort_unstable();
    pairs.dedup_by(|x, y| x.0 == y.0 && x.1 == y.1);
    // 文字 l の行 (符号 s) での増分: x_j = 1 (偶数) は a > 0 のとき a、x_j = 0 (奇数) は a < 0 のとき −a
    let inc = |l: u32, a: f64| if l % 2 == 0 { a.max(0.0) } else { (-a).max(0.0) };
    let (mut ca, mut cb_lits, mut cb_edges, mut cb_other, mut cc, mut cd) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let (mut new_all, mut new_a, mut new_b, mut new_d) = (0usize, 0usize, 0usize, 0usize);
    let mut cd_share = 0usize; // (d) のうち 2 列がどこかの行にそろって入る組
    let (mut mark, mut stamp, mut d_mid) = (vec![0u32; n], 0u32, [0usize; 4]);
    for &(u, v, fresh) in &pairs {
        new_all += fresh as usize;
        if g.adjacent(u, v) {
            ca += 1;
            new_a += fresh as usize;
            continue;
        }
        let (ju, jv) = ((u / 2) as usize, (v / 2) as usize);
        let (ru, rv) = (&col_rows[ju], &col_rows[jv]);
        let (mut x, mut y) = (0usize, 0usize);
        let (mut b_kind, mut inf_share, mut share) = (0u8, false, false);
        while x < ru.len() && y < rv.len() {
            if ru[x].0 < rv[y].0 {
                x += 1;
            } else if ru[x].0 > rv[y].0 {
                y += 1;
            } else {
                share = true;
                let i = ru[x].0 as usize;
                for (si, sign) in [1.0, -1.0].into_iter().enumerate() {
                    let (fin, slack, nl, np) = side[i][si];
                    if nl == 0 && !fin {
                        continue;
                    }
                    if !fin {
                        if nl >= 2 {
                            inf_share = true;
                        }
                        continue;
                    }
                    let b = if si == 0 { prob.row_up[i] } else { -prob.row_lo[i] };
                    let tol = 1e-9 * (1.0 + b.abs());
                    let (iu, iv) = (inc(u, sign * ru[x].1), inc(v, sign * rv[y].1));
                    if iu > 0.0 && iv > 0.0 && iu + iv > slack + tol {
                        let k = if nl > MAX_ROW_LITS_DIAG { 1 } else if np > MAX_ROW_EDGES_DIAG { 2 } else { 3 };
                        b_kind = if b_kind == 0 { k } else { b_kind.min(k) };
                    }
                }
                x += 1;
                y += 1;
            }
        }
        match b_kind {
            1 => cb_lits += 1,
            2 => cb_edges += 1,
            3 => cb_other += 1,
            _ if inf_share => cc += 1,
            _ => {
                cd += 1;
                cd_share += share as usize;
                new_d += fresh as usize;
                // 2 段の連鎖 (x_i = v → 列 k → x_j = w) の候補: 2 列とそれぞれ行を共にする列 k があるか、その k の種類
                stamp += 1;
                for &(i, _) in ru {
                    for &(k, _) in &prob.rows[i as usize] {
                        mark[k] = stamp;
                    }
                }
                let mut kind = 3usize; // 0: 0-1 列, 1: 一般整数, 2: 連続, 3: 無し
                'mid: for &(i, _) in rv {
                    for &(k, _) in &prob.rows[i as usize] {
                        if mark[k] == stamp && k != ju && k != jv {
                            let kk = if binary[k] { 0 } else if prob.is_int[k] { 1 } else { 2 };
                            kind = kind.min(kk);
                            if kind == 0 {
                                break 'mid;
                            }
                        }
                    }
                }
                d_mid[kind] += 1;
            }
        }
        if b_kind != 0 {
            new_b += fresh as usize;
        }
    }
    cap_lens.sort_unstable_by(|a, b| b.cmp(a));
    cap_lens.truncate(20);
    eprintln!(
        "IMPLCLASS {{\"n\":{n},\"rows\":{m},\"edges_raw\":{},\"edges_inherited\":{e_init},\"pairs\":{},\"pairs_new\":{new_all},\"a_in_graph\":{ca},\"b_row_over_lits\":{cb_lits},\"b_row_over_edges\":{cb_edges},\"b_row_other\":{cb_other},\"c_inf_row\":{cc},\"d_chain\":{cd},\"d_chain_share_row\":{cd_share},\"d_mid_bin_int_cont_none\":{d_mid:?},\"new_a\":{new_a},\"new_b\":{new_b},\"new_d\":{new_d},\"graph_edges\":{},\"row_sides_lits_hist\":{:?},\"row_sides_over_lits\":{cap_lits},\"row_sides_over_edges\":{cap_edges},\"row_sides_inf\":{inf_rows},\"cap_row_lens_top\":{:?},\"secs\":{:.2}}}",
        edges.len(),
        pairs.len(),
        g.num_edges(),
        &hist[..7],
        cap_lens,
        t0.elapsed().as_secs_f64()
    );
}
const MAX_ROW_LITS_DIAG: usize = 2000;
const MAX_ROW_EDGES_DIAG: u64 = 20_000;

/// 診断 `ENOMOTO_MIP_DEBUG_CLQTAB` (クリーク表の持ち方の見直しの数え上げ、analysis/clqprb_20261112.md §11): 前処理の後の問題で、
/// 衝突グラフ ([`clique::CliqueGraph`]) の辺の数・作る時間・行の上限で落ちる行と組、含意の組 (`conflict_edges`) のうち行の辺と重なる数、
/// 残りの含意の組を貪欲にクリークにまとめたときの個数と総文字数を数え、`CLQTAB {json}` の 1 行を出す (求解は変えない)
pub(crate) fn clqtab_census(prob: &MipProblem, tag: &str) {
    let n = prob.n;
    let (lo, up) = (&prob.col_lo, &prob.col_up);
    let binary: Vec<bool> = (0..n).map(|j| prob.is_int[j] && lo[j] == 0.0 && up[j] == 1.0).collect();
    let t0 = std::time::Instant::now();
    let g = clique::CliqueGraph::build_with(n, &prob.rows, &prob.row_lo, &prob.row_up, &binary, lo, up, &prob.conflict_edges);
    let t_full = t0.elapsed().as_secs_f64();
    let t1 = std::time::Instant::now();
    let g0 = clique::CliqueGraph::build(n, &prob.rows, &prob.row_lo, &prob.row_up, &binary, lo, up);
    let t_rows = t1.elapsed().as_secs_f64();
    let cap = clique::CliqueGraph::row_cap_stats(&prob.rows, &prob.row_lo, &prob.row_up, &binary, lo, up);
    // 含意の組 (重複・固定の列・同じ列を除く)
    let mut imp: Vec<(u32, u32)> = prob
        .conflict_edges
        .iter()
        .filter(|&&(a, b)| a / 2 != b / 2 && binary[(a / 2) as usize] && binary[(b / 2) as usize])
        .map(|&(a, b)| (a.min(b), a.max(b)))
        .collect();
    imp.sort_unstable();
    imp.dedup();
    let n_imp = imp.len();
    let rest: Vec<(u32, u32)> = imp.iter().copied().filter(|&(a, b)| !g0.adjacent(a, b)).collect();
    let n_rest = rest.len();
    // 残りの組を貪欲にクリークにまとめる: 次数の大きい文字から、まだ覆っていない隣を全体のグラフ g で互いに隣なものから足していく
    let t2 = std::time::Instant::now();
    let mut adj: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    for &(a, b) in &rest {
        adj.entry(a).or_default().push(b);
        adj.entry(b).or_default().push(a);
    }
    let rest_set: std::collections::HashSet<(u32, u32)> = rest.iter().copied().collect();
    let mut covered: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::with_capacity(n_rest);
    let mut order: Vec<u32> = adj.keys().copied().collect();
    order.sort_by_key(|u| (std::cmp::Reverse(adj[u].len()), *u));
    let (mut n_cl, mut n_lits, mut max_cl, mut timed_out) = (0usize, 0usize, 0usize, false);
    let key = |a: u32, b: u32| (a.min(b), a.max(b));
    let tmax = tunable!("ENOMOTO_T_MIP_CLQTAB_TIME", 60.0, f64);
    'outer: for &u in &order {
        loop {
            if t2.elapsed().as_secs_f64() > tmax {
                timed_out = true;
                break 'outer;
            }
            let nb: Vec<u32> = adj[&u].iter().copied().filter(|&v| !covered.contains(&key(u, v))).collect();
            if nb.is_empty() {
                break;
            }
            let mut cl = vec![u, nb[0]];
            for &w in &nb[1..] {
                if cl.len() >= 256 {
                    break;
                }
                if cl[1..].iter().all(|&c| g.adjacent(w, c)) {
                    cl.push(w);
                }
            }
            let mut newly = 0usize;
            for x in 0..cl.len() {
                for y in (x + 1)..cl.len() {
                    let k = key(cl[x], cl[y]);
                    if rest_set.contains(&k) && covered.insert(k) {
                        newly += 1;
                    }
                }
            }
            if newly == 0 {
                // 進まない (覆いの印が rest にない組だけ): u の残りを 1 組ずつ覆う
                for &v in &nb {
                    covered.insert(key(u, v));
                    n_cl += 1;
                    n_lits += 2;
                }
                break;
            }
            n_cl += 1;
            n_lits += cl.len();
            max_cl = max_cl.max(cl.len());
        }
    }
    eprintln!(
        "CLQTAB {{\"at\":\"{tag}\",\"n_bin\":{},\"edges_full\":{},\"edges_rows\":{},\"build_full_s\":{t_full:.2},\"build_rows_s\":{t_rows:.2},\"adj_mb\":{:.1},\"rows_over_lits\":{},\"pairs_over_lits\":{},\"rows_over_edges\":{},\"pairs_over_edges\":{},\"row_pairs_total\":{},\"impl_pairs\":{n_imp},\"impl_not_rows\":{n_rest},\"impl_cliques\":{n_cl},\"impl_clique_lits\":{n_lits},\"impl_clique_max\":{max_cl},\"cover_s\":{:.2},\"cover_timeout\":{timed_out}}}",
        binary.iter().filter(|&&b| b).count(),
        g.num_edges(),
        g0.num_edges(),
        (g.num_edges() * 2 * 4) as f64 / 1e6,
        cap[0],
        cap[1],
        cap[2],
        cap[3],
        cap[4],
        t2.elapsed().as_secs_f64()
    );
}

/// 文字と重み (正) の並びを、衝突グラフ `g` で重みの大きい順に貪欲にクリークに分ける (1 文字あたり先頭 64 個のクリークまで試す)。
/// 重みの和が `own` を超える組は使わない (行そのものから出る組)。(クリークごとの最大の和, 文字ごとの「その文字を除いたときの和の減り」)。
/// `lits` は重みの降順に並べ替える
fn clq_partition(g: &clique::CliqueGraph, lits: &mut [(f64, u32)], own: f64) -> (f64, Vec<f64>) {
    lits.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut cls: Vec<Vec<usize>> = Vec::new();
    let mut of = vec![0usize; lits.len()];
    for (q, &(w, l)) in lits.iter().enumerate() {
        let mut put = None;
        for (c, cl) in cls.iter().enumerate().take(64) {
            if cl.iter().all(|&o| w + lits[o].0 <= own && g.adjacent(l, lits[o].1)) {
                put = Some(c);
                break;
            }
        }
        match put {
            Some(c) => {
                cls[c].push(q);
                of[q] = c;
            }
            None => {
                of[q] = cls.len();
                cls.push(vec![q]);
            }
        }
    }
    let total: f64 = cls.iter().map(|cl| lits[cl[0]].0).sum();
    let dec: Vec<f64> = (0..lits.len())
        .map(|q| {
            let cl = &cls[of[q]];
            if cl[0] == q { lits[q].0 - cl.get(1).map_or(0.0, |&o| lits[o].0) } else { 0.0 }
        })
        .collect();
    (total, dec)
}

/// 試作 `ENOMOTO_MIP_CLQACT` (既定では使わない。analysis/clqprb_20261112.md §12): 前処理の後の問題で、行の活動量にクリーク分割を使って
/// (1) 片側の行の 0-1 列の係数を締め (x_k の文字を偽にしたときの最大の活動量が右辺を下回るなら、その差 d だけ係数と右辺を減らす)、
/// (2) 連続列・一般の整数列の境界を締める (最小の活動量から)。行は消さない。
/// 使う衝突の組は前処理の後の**元の行**から出る組 ([`clique::CliqueGraph::build`]) で、締める行そのものから出る組は使わない
/// (プロービングの含意は出どころが分からないので使わない)。係数を締めても行から出る組は減らない (右辺と係数が同じだけ減るので、
/// 含む組は同じ、含まない組は増えるだけ) ので、ほかの行の元の組はずっと成り立ち、順にかけても整数の実行可能な点の集合は変わらない。
/// (締めた係数の数, 締めた境界の数) を返す
pub(crate) fn clqact_tighten(prob: &mut MipProblem, verbose: bool) -> (usize, usize) {
    let t0 = std::time::Instant::now();
    let n = prob.n;
    let binary: Vec<bool> = (0..n).map(|j| prob.is_int[j] && prob.col_lo[j] == 0.0 && prob.col_up[j] == 1.0).collect();
    let g = clique::CliqueGraph::build(n, &prob.rows, &prob.row_lo, &prob.row_up, &binary, &prob.col_lo, &prob.col_up);
    let maxlits = tunable!("ENOMOTO_T_MIP_CLQACT_MAXLITS", 1000usize, usize);
    let (mut ncoef, mut nbnd) = (0usize, 0usize);
    // 行 i の `sign * a x <= b` の形について、(固定・0-1 以外の列の (列, 符号つき係数), 増える側の文字, 減る側の文字, 0-1 以外の列の幅の和, 文字が全部偽の最大の活動量, 最小の活動量) を作る
    #[allow(clippy::type_complexity)]
    let parts = |prob: &MipProblem, i: usize, sign: f64| -> Option<(Vec<(usize, f64)>, Vec<(f64, u32)>, Vec<(f64, u32)>, f64, f64, f64)> {
        let (lo, up) = (&prob.col_lo, &prob.col_up);
        let (mut base_max, mut base_min, mut span) = (0.0, 0.0, 0.0);
        let (mut ul, mut dl, mut other) = (Vec::new(), Vec::new(), Vec::new());
        for &(j, a0) in &prob.rows[i] {
            let a = sign * a0;
            if lo[j] == up[j] {
                base_max += a * lo[j];
                base_min += a * lo[j];
            } else if binary[j] {
                if a > 0.0 {
                    ul.push((a, 2 * j as u32));
                    dl.push((a, 2 * j as u32 + 1));
                    base_min += a;
                } else {
                    ul.push((-a, 2 * j as u32 + 1));
                    dl.push((-a, 2 * j as u32));
                    base_max += a;
                }
            } else {
                let (mx, mn) = if a > 0.0 { (a * up[j], a * lo[j]) } else { (a * lo[j], a * up[j]) };
                if !mx.is_finite() || !mn.is_finite() {
                    return None;
                }
                base_max += mx;
                base_min += mn;
                span += mx - mn;
                other.push((j, a));
            }
        }
        (ul.len() >= 2 && ul.len() <= maxlits).then_some((other, ul, dl, span, base_max, base_min))
    };
    for i in 0..prob.m {
        // (1) 係数の締め付け (片側の行だけ)
        let side = match (prob.row_lo[i].is_finite(), prob.row_up[i].is_finite()) {
            (false, true) => Some(1.0),
            (true, false) => Some(-1.0),
            _ => None,
        };
        if let Some(sign) = side {
            for _ in 0..32 {
                let b = if sign > 0.0 { prob.row_up[i] } else { -prob.row_lo[i] };
                let tol = 1e-9 * (1.0 + b.abs());
                let Some((_, mut ul, _, span, base_max, _)) = parts(prob, i, sign) else { break };
                let (tot, dec) = clq_partition(&g, &mut ul, b - (base_max - span) + tol);
                let max_new = base_max + tot;
                if max_new <= b + tol {
                    break;
                }
                // 締め幅 d = b - (l を偽にしたときの最大の活動量) が最大の文字
                let best = (0..ul.len()).map(|q| (b - (max_new - dec[q]), q)).filter(|&(d, q)| d > 1e-6 * (1.0 + b.abs()) && d < ul[q].0 - 1e-9).max_by(|x, y| x.0.total_cmp(&y.0));
                let Some((d, q)) = best else { break };
                let l = ul[q].1;
                let j = (l / 2) as usize;
                let pos_lit = l % 2 == 0; // 文字が x_j (符号つき係数 > 0) か 1 - x_j (< 0) か
                // 符号つきの形で: x_j の文字なら a' = a - d、b' = b - d。1 - x_j の文字なら a' = a + d、b は同じ
                let k = prob.rows[i].iter().position(|t| t.0 == j).unwrap();
                let a_signed = sign * prob.rows[i][k].1;
                let (a2, b2) = if pos_lit { (a_signed - d, b - d) } else { (a_signed + d, b) };
                prob.rows[i][k].1 = sign * a2;
                if let Some(e) = prob.cols[j].iter_mut().find(|e| e.0 == i) {
                    e.1 = sign * a2;
                }
                if sign > 0.0 {
                    prob.row_up[i] = b2;
                } else {
                    prob.row_lo[i] = -b2;
                }
                ncoef += 1;
            }
        }
        // (2) 0-1 以外の列の境界 (両方の向き)
        for (sign, b, b_other) in [(1.0, prob.row_up[i], -prob.row_lo[i]), (-1.0, -prob.row_lo[i], prob.row_up[i])] {
            if !b.is_finite() {
                continue;
            }
            let Some((other, _, mut dl, span, _, base_min)) = parts(prob, i, sign) else { continue };
            if other.is_empty() {
                continue;
            }
            let old_dn: f64 = dl.iter().map(|t| t.0).sum();
            let own_dn = if b_other.is_finite() { b_other + base_min + span + 1e-9 * (1.0 + b_other.abs()) } else { f64::INFINITY };
            let (new_dn, _) = clq_partition(&g, &mut dl, own_dn);
            let min_new = base_min - new_dn;
            if min_new <= base_min - old_dn + 1e-9 * (1.0 + b.abs()) {
                continue;
            }
            for &(j, a) in &other {
                let cmin = if a > 0.0 { a * prob.col_lo[j] } else { a * prob.col_up[j] };
                let v = (b - (min_new - cmin)) / a;
                if a > 0.0 {
                    let mut nv = v + 1e-9 * (1.0 + v.abs());
                    if prob.is_int[j] {
                        nv = (v + 1e-6).floor();
                    }
                    if nv < prob.col_up[j] - 1e-7 * (1.0 + prob.col_up[j].abs()) && nv >= prob.col_lo[j] {
                        prob.col_up[j] = nv;
                        nbnd += 1;
                    }
                } else {
                    let mut nv = v - 1e-9 * (1.0 + v.abs());
                    if prob.is_int[j] {
                        nv = (v - 1e-6).ceil();
                    }
                    if nv > prob.col_lo[j] + 1e-7 * (1.0 + prob.col_lo[j].abs()) && nv <= prob.col_up[j] {
                        prob.col_lo[j] = nv;
                        nbnd += 1;
                    }
                }
            }
        }
    }
    if verbose {
        eprintln!("MIP: presolve: CLQACT tightened {ncoef} coefficients and {nbnd} bounds ({} edges from rows, {:.2}s)", g.num_edges(), t0.elapsed().as_secs_f64());
    }
    (ncoef, nbnd)
}

/// [`clqact_tighten`] を使うか (`ENOMOTO_MIP_CLQACT`)。
pub(crate) fn clqact_on() -> bool {
    env_str!("ENOMOTO_MIP_CLQACT").is_some_and(|v| v != "0")
}

/// 試作 `ENOMOTO_MIP_CLQPRB` (既定では使わない。analysis/clqprb_20261112.md): 含意を集めるプロービング ([`probe`]) で、見つけた含意
/// (クリーク表の 2 文字のクリークにあたる) を後の列の伝播に使い (HiGHS の probing はクリーク表で伝播する)、含意の本数の上限を広げ、
/// 試す回数の予算と時間の上限を広げる
pub(crate) fn clqprb_on() -> bool {
    clqprb_val().is_some()
}

/// `ENOMOTO_MIP_CLQPRB` の値 (`0`・`off` なら `None`)。[`CLQPRB_DEFAULT_ON`] が真なら、付けていないときは `1dgrs` (gate2 候補 2 の形)
fn clqprb_val() -> Option<&'static str> {
    if CLQPRB_DEFAULT_ON {
        env_def!("ENOMOTO_MIP_CLQPRB", "1dgrs")
    } else {
        env_str!("ENOMOTO_MIP_CLQPRB").filter(|v| *v != "0" && *v != "off")
    }
}

/// `ENOMOTO_MIP_CLQPRB` を既定で入れるか (既定化するときに真にする。analysis/gate2_20261117/clqprb_s/README.md)
const CLQPRB_DEFAULT_ON: bool = true;

/// [`clqprb_on`] の値に文字 `c` を含むか (`d`: 何も固定できなければ増やした含意を捨てる)。
pub(crate) fn clqprb_has(c: char) -> bool {
    clqprb_val().is_some_and(|v| v.contains(c))
}

thread_local! {
    /// `ENOMOTO_MIP_CLQPRB` の `n`・`g` (試作、analysis/prbimp_20261113.md): 既定の形でプロービングを回す間だけ立てる
    /// (このあいだ [`probe`] は `CLQPRB` を使わない)
    static CLQPRB_SUPPRESS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// `ENOMOTO_MIP_CLQPRB` の `g`: 既定の形で何か得られたので、含意を使う形でやり直す間だけ立てる (このあいだ `g` の判定をしない)
    static CLQPRB_FORCE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// `ENOMOTO_MIP_CLQPRB` の `g`: この求解で一度含意を使う形に切り替えたら、後のプロービング (前処理のかけ直し・再スタート) でも判定せずに使う
    /// (neos8 は 2 回目のプロービングで既定の形では同値な組が 0 組、含意を使うと 1152 組)。求解の始めに戻す
    static CLQPRB_STICKY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// `CLQPRB` の `r`: 含意を使う形が既定の形より多く得ず、既定の結果に戻した。その求解の残りでは既定の形を使う
    static CLQPRB_REVERTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 最後の [`probe`] の成果 (固定, 締め付け, 同値な組)。`ENOMOTO_MIP_CLQPRB` の `g` の判定に使う
    static PROBE_GAIN: std::cell::Cell<(usize, usize, usize)> = const { std::cell::Cell::new((0, 0, 0)) };
}

/// 試作 `ENOMOTO_MIP_PREPAR` (既定では使わない。analysis/prepar_20261107.md): 前処理の並列化。値の文字で選ぶ
/// (`p`: 含意を集めるプロービング ([`probe`]) の候補を投機的に並べる、`e`: プロービングの同値な組の巡を並べる、
/// `l`: LP による固定 ([`lp_probe`]) の 1 巡の列の試しを並べる)
/// 試作 `ENOMOTO_MIP_LPPROBE_TICKS` (analysis/scipsym_20261120/ 14・15 節、既定では使わない): LP による固定 ([`lp_probe`]) で固定がまだ 0 のとき、
/// 子の LP に使った決定的な手間 (`crate::ticks`、窓で並べた写しの分も足す) が最初の LP の手間の `ENOMOTO_T_MIP_LP_PROBE_GIVEUP_TIME_MULT` (既定 1) 倍を
/// 越えたらあきらめる (手間の既定 `ENOMOTO_MIP_PRESOLVE_WORK` のときだけ)。反復 x (行 + 列) の数えは 1 反復の重い密な問題 (cod105) で軽く出る
/// `ENOMOTO_MIP_LPPROBE_FIX=<文字>` (analysis/lpprobe_20261126、未設定は `d`): 前処理の LP プロービング ([`lp_probe`]) の直し。
/// - `d` (結果を変えない): 子の LP の厳密な DSE 重みを使い回す ([`crate::simplex::slope_intercept_dual::lpprobe_dse_scope`])
/// - `c` (結果が変わりうる): 子の LP 1 本の反復の上限 `per_lp` に、ここまでの効き目 (試した列のうち固定が出た割合) のシグモイドを掛ける。
///   効き目の対数オッズ (ラプラスの数え方) L = ln((固定 + 1) / (試した列 − 固定 + 1)) で 2σ(L) = 2 (固定 + 1) / (試した列 + 2) (1 で頭打ち)。
///   まだ何も試していなければ 1 (今と同じ)、固定のないまま列を試すほど縮む (2 列で 1/2、6 列で 1/4)。窓では窓の始めの値を使う (数えた量だけで決まる)。
///   P では結果を生む子 (実行不能) は上限の 0.55 倍以内で終わり、上限まで走る子は何も生まない (analysis/lpprobe_20261126 1.2 節)
/// **未設定は `d` で既定で使う** (2026-11-26 に利用者の承認で既定化、analysis/lpprobe_20261126 5 節)。`=0`・`off` で切る。`c` は試作 (既定オフ)
pub(crate) fn lpprobe_fix(c: char) -> bool {
    env_def!("ENOMOTO_MIP_LPPROBE_FIX", "d").is_some_and(|v| v.contains(c))
}

/// 試作 `ENOMOTO_MIP_LPPROBE_FIX` の `c` ([`lpprobe_fix`]): 子の LP の反復の上限 `per_lp` に、試した列 `tried` のうち固定 `fixed` の効き目の
/// シグモイド 2σ(ln((fixed + 1) / (tried − fixed + 1))) = 2 (fixed + 1) / (tried + 2) を掛ける (1 で頭打ち、最低 1 反復)。`c` でなければ `per_lp`
fn lpp_cap(per_lp: u64, tried: usize, fixed: usize) -> u64 {
    if !lpprobe_fix('c') {
        return per_lp;
    }
    let fixed = fixed.min(tried);
    let s = (2.0 * (fixed as f64 + 1.0) / (tried as f64 + 2.0)).min(1.0);
    ((per_lp as f64 * s).round() as u64).max(1)
}

/// 診断 `ENOMOTO_MIP_LPPROBE_DIAG` (analysis/lpprobe_20261126): 前処理の LP プロービングの子の LP ごとに、状態・反復・秒・目的値を stderr に出す
pub(crate) fn lpprobe_diag() -> bool {
    env_str!("ENOMOTO_MIP_LPPROBE_DIAG").is_some()
}
pub(crate) fn lpprobe_ticks() -> bool {
    env_str!("ENOMOTO_MIP_LPPROBE_TICKS").is_some_and(|v| v != "0")
}

pub(crate) fn prepar_has(c: char) -> bool {
    env_def!("ENOMOTO_MIP_PREPAR", "ple").is_some_and(|v| v.contains(c))
}

thread_local! {
    /// `ENOMOTO_MIP_PREPAR` で使うスレッドの数 (最初の求解の `params.threads`。サブ MIP・再スタートでは 1)。1 本でも窓で試す
    /// (結果をスレッドの数によらずそろえるため)
    static PREPAR_THREADS: std::cell::Cell<usize> = const { std::cell::Cell::new(1) };
}

/// 試作 `ENOMOTO_MIP_PRB_TABLE=<文字>` (調べ C の 13 節、analysis/primal_20261117、既定では使わない): HiGHS 1.15.1 の前処理のプロービング
/// (`HighsImplications::runProbing`・`HighsCliqueTable::processNewEdge`) にならう。HiGHS は MIT ライセンス (Copyright (c) 2024 HiGHS)。
/// 規則を読んで当方の作りで書き直したもので、コードは写していない (これまでの HiGHS の移植と同じ扱い)。
/// - `a` (P1): プロービングの前に行から衝突の表 (2 値の文字の組) を作り、1 列のプロービングで見つけた含意 (x = v ⇒ y = w を「文字 (x = v) と
///   文字 (y = 1 − w) の衝突」として) を表に足すたびに、`processNewEdge` の規則を回す: 新しい衝突 (a, b) について、(¬a, b) も衝突なら b は偽 (固定)、
///   (¬b, a) も衝突なら a は偽、(¬a, ¬b) も衝突なら a と b のちょうど一方が真 (同値な組として代入)。含意は後の伝播にも使う (`ENOMOTO_MIP_PROBE_CLIQUE` と同じ)
/// - `b` (P2): プロービングの後の前処理のかけ直しを `ENOMOTO_T_MIP_PRB_TABLE_ROUNDS` (8) 回まで、行の減り方によらず使う
pub(crate) fn prb_table(c: char) -> bool {
    static V: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    V.get_or_init(|| env_str!("ENOMOTO_MIP_PRB_TABLE").unwrap_or("").to_string()).contains(c)
}

/// 試作 `ENOMOTO_MIP_PRBWORK` (既定では使わない。analysis/probework_20261120): 含意を集めるプロービング ([`probe`]) が、固定・締め付け・
/// 同値な組を 1 つも得ないまま「何も見つからない間の上限」(手間 `ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE0` か時間の安全網) で打ち切られたら、
/// その求解の再スタートの後のプロービングを回さず、再スタートの問題に前の求解の含意を引き継ぐ (再スタートの問題は前の問題を狭めたものなので、
/// 含意はそのまま成り立つ)。後のプロービングが何か得れば印を消す。
/// irp は根と 3 回の再スタートの後のプロービングが毎回この上限で止まり、成果 0 のまま 1.2 秒ずつ使っていた。
/// 候補を最後まで試して成果 0 の回 (neos-860300 のかけ直しの中・roll3000 の根) は印を付けない (roll3000 は再スタートの後に 12 列、
/// neos-860300 は 7・9・6 列を固定する)
pub(crate) fn prbwork_on() -> bool {
    env_str!("ENOMOTO_MIP_PRBWORK").is_some_and(|v| v != "0")
}

thread_local! {
    /// `ENOMOTO_MIP_PRBWORK` ([`prbwork_on`]): この求解の最後のプロービングが成果 0 のまま上限で打ち切られた (最初の求解の始めに消す)
    static PRBWORK_DRY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// [`prbwork_on`] で、再スタートの後のプロービングを省くか (サブ MIP は対象外)
pub(crate) fn prbwork_skip_restart(submip: bool) -> bool {
    prbwork_active(submip) && PRBWORK_DRY.with(|f| f.get())
}

/// [`prbwork_on`] をこのプロービングに使うか。サブ MIP (親と同じ前処理で解くものを含む) は対象外
fn prbwork_active(submip: bool) -> bool {
    prbwork_on() && !submip && !PRESOLVE_IN_SUBMIP.with(|f| f.get())
}

/// 試作 `ENOMOTO_MIP_PROBREP` (既定では使わない。analysis/probrep_20261123): 同じ求解の前のプロービングが何も得ず (固定・締め付け・同値な組が 0)、
/// その後の変化が行だけ (列の数・境界・整数の印が前のプロービングに渡した問題と同じ) なら、次のプロービングを回さず、前の回の含意と
/// 含意による境界をそのまま使う。行だけの変化 (冗長な行の削除・平行な行の併合・等式による疎化) は同じ実行可能集合の書き直しで、
/// 列の定義域は同じなので、前の回が何も得なかったプロービングをやり直す理由がない (新しい閾値なし)。
/// nursesched-sprint02 は根の前処理で、成果 0 の 1.3 秒のプロービングを疎化の後にもう一度回していた (固定 0・含意の数も同じ)。
/// 再スタートの問題は前の求解の問題 (前処理の後) を狭めたものなので、再スタートの前処理で列が 1 本も消えず境界も変わらなければ、
/// 前の求解の最後の成果 0 の回の含意がそのまま成り立ち、ここでも省く (supportcase18 は再スタートで新たな固定が 0 で、3 回の再スタートの後を省く)。
/// `ENOMOTO_MIP_PRBWORK` (再スタートの後、前の回が成果 0 のまま上限で打ち切られたら、列が変わっていても回さない) とは別の判定で、両方つければ
/// どちらかが当てはまれば省く。`f` を含めば成果があった回も控える (fuzz 用の強制)
pub(crate) fn probrep_on() -> bool {
    env_str!("ENOMOTO_MIP_PROBREP").is_some_and(|v| v != "0")
}

/// [`probrep_on`]: 成果 0 だったプロービングに渡した問題の列の状態と、その回の含意・含意による境界
struct ProbRepSnap {
    n: usize,
    /// 列の対応が変わっていないことの確かめに、境界・整数の印に加えて目的係数も比べる
    cost: Vec<f64>,
    col_lo: Vec<f64>,
    col_up: Vec<f64>,
    is_int: Vec<bool>,
    edges: Vec<(u32, u32)>,
    implied: Vec<(u32, bool, u32, bool, f64)>,
}

thread_local! {
    /// [`probrep_on`]: この求解の最後のプロービングが成果 0 なら、その回の写し (成果があれば消す。最初の求解の始めに消す)
    static PROBREP_LAST: std::cell::RefCell<Option<ProbRepSnap>> = const { std::cell::RefCell::new(None) };
}

fn probe(prob: &mut MipProblem, verbose: bool, submip: bool) -> Option<bool> {
    let _pt = pretime::guard("probe");
    if enum_presolve_mode().is_some() && enum_where() == "loop" && !submip && !ENUM_LOOP_IN.with(|f| f.get()) {
        ENUM_LOOP_IN.with(|f| f.set(true));
        let r = enum_probe_loop(prob, verbose, submip);
        ENUM_LOOP_IN.with(|f| f.set(false));
        return r;
    }
    // `ENOMOTO_MIP_PROBREP` ([`probrep_on`]): 前の回が成果 0 で、列の状態が同じなら回さない
    if probrep_on() && !submip && !PRESOLVE_IN_SUBMIP.with(|f| f.get()) {
        let same = PROBREP_LAST.with(|c| {
            c.borrow().as_ref().filter(|sn| sn.n == prob.n && sn.cost == prob.cost && sn.col_lo == prob.col_lo && sn.col_up == prob.col_up && sn.is_int == prob.is_int).map(|sn| (sn.edges.clone(), sn.implied.clone()))
        });
        if let Some((edges, implied)) = same {
            if verbose {
                eprintln!(
                    "MIP: probing: skipped (PROBREP: the last probing found nothing and only rows changed since; {} implications and {} implied bounds kept)",
                    edges.len(),
                    implied.len()
                );
            }
            prob.conflict_edges = edges;
            prob.implied_bounds = implied;
            PROBE_GAIN.with(|f| f.set((0, 0, 0)));
            PROBE_FIXED.with(|f| f.set(0));
            return None;
        }
    }
    use domain::Domain;
    // `ENOMOTO_MIP_PRBWORK` ([`prbwork_on`]): 再スタートの後で、前のプロービングが成果 0 のまま打ち切られていれば回さない (引き継いだ含意 `conflict_edges` はそのまま残す)
    if RESTART_PRESOLVE.with(|f| f.get()) && prbwork_skip_restart(submip) {
        if verbose {
            eprintln!("MIP: probing: skipped after the restart (PRBWORK: the last probing found nothing within its limit; {} implications kept)", prob.conflict_edges.len());
        }
        PROBE_GAIN.with(|f| f.set((0, 0, 0)));
        PROBE_FIXED.with(|f| f.set(0));
        return None;
    }
    let t0 = std::time::Instant::now();
    let c0 = cap_clock();
    let n = prob.n;
    let nnz = prob.nnz;
    // 診断用 (`ENOMOTO_MIP_DEBUG_DUMP_PROBE=接頭辞`): プロービングに渡された問題を `接頭辞_番号.mps`、見つけた同値な組を `接頭辞_番号.pairs` に書く
    let dump_id = env_str!("ENOMOTO_MIP_DEBUG_DUMP_PROBE").filter(|_| !submip).map(|f| {
        static CNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = format!("{f}_{}", CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
        dump_mps(prob, &format!("{id}.mps"));
        eprintln!("MIP: probing: dumped the problem to {id}.mps (offset {}, restart {})", prob.offset, RESTART_PRESOLVE.with(|f| f.get()));
        id
    });
    // 旧来の上限 (サブ MIP と `ENOMOTO_MIP_NO_PROBE_ADAPTIVE`): 手間は非零数の 20 倍、時間は 0.5 秒 (固定・締め付けが出ていれば
    // `ENOMOTO_T_MIP_PROBE_TIME_FOUND` 秒)。
    // 大きくすると (2000 倍・1 秒・3 秒) 30n20b8 は 7971 列を全部試して 3241 列を固定し根の LP が 43.3 -> 122.8
    // (HiGHS 123.0)、neos-860300 も 2035 -> 2210 になるが、40 問では 19 問 28.14 -> 18 問 28.08 (neos-860300 が
    // 2 回とも時間切れ、30n20b8 はそれでも解が見つからない) で既定にはしない
    let work_cap = (tunable!("ENOMOTO_T_MIP_PROBE_WORK", 20.0, f64) * nnz as f64) as u64 + 100_000;
    let time_cap = tunable!("ENOMOTO_T_MIP_PROBE_TIME", 0.5, f64);
    let time_cap_found = tunable!("ENOMOTO_T_MIP_PROBE_TIME_FOUND", 0.5, f64);
    let mut dom = Domain::new(prob);
    if !dom.propagate(prob) {
        return Some(false);
    }
    // 行の多い列から
    let mut cands: Vec<usize> = (0..n).filter(|&j| prob.is_int[j] && dom.lo[j] == 0.0 && dom.up[j] == 1.0).collect();
    cands.sort_by_key(|&j| std::cmp::Reverse(prob.cols[j].len()));
    let work0 = dom.debug_work();
    let wcut0 = domain::work_cuts();
    let (mut probed, mut fixed, mut tightened) = (0usize, 0usize, 0usize);
    // 含意 (`ENOMOTO_MIP_NO_PROBE_IMPLICATIONS` で集めない): x_j = v で 0-1 列 x_k が固定されれば、
    // 「x_j = v かつ x_k が固定と逆の値」は成り立たない (衝突の組)。上限は非零数の 10 倍
    let want_impl = env_str!("ENOMOTO_MIP_NO_PROBE_IMPLICATIONS").is_none();
    let want_implied = env_str!("ENOMOTO_MIP_IMPLBD_CUTS").is_some() || env_str!("ENOMOTO_MIP_NO_IMPLBD_CONT").is_none();
    // `ENOMOTO_T_MIP_PROBE_EDGE_CAP_MULT` (試作、CLQPRB): 含意の本数の上限の倍率 (既定 10)
    // `ENOMOTO_MIP_CLQPRB` (試作、既定では使わない。analysis/clqprb_20261112.md、[`clqprb_on`]): 上限を非零数の
    // `ENOMOTO_T_MIP_CLQPRB_EDGE_MULT` 倍 (既定 40) と `ENOMOTO_T_MIP_CLQPRB_EDGE_MAX` 本 (既定 600 万) の小さい方にする (非零数の 10 倍は下回らない)。
    // 上限に届いたら重複を除いて詰める (2 巡目は同じ含意をもう一度集めるので、重複込みで数えると上限に早く届く)。mzzv11 は 10 倍 (136 万本) では
    // 足りず、後ろの候補の含意が落ちて、含意を使う伝播で見つかるはずの固定 (HiGHS の probing の固定の大半) を逃していた
    let clqprb = clqprb_on() && !submip && !CLQPRB_SUPPRESS.with(|f| f.get()) && !CLQPRB_REVERTED.with(|f| f.get());
    // `ENOMOTO_MIP_CLQPRB` の `g` (試作、analysis/prbimp_20261113.md): まず既定の形でプロービングを回し、固定・締め付け・同値な組が
    // `ENOMOTO_T_MIP_CLQPRB_G_MIN` (既定 1) 個以上出たときだけ、問題を元に戻して含意を使う形でやり直す。何も得られない問題
    // (roll3000・cbs-cta・air03・nw04 など。含意を使っても固定は出ず、含意の数・同値な組・プロービングの時間だけが変わって後の流れが変わる) では
    // 既定とまったく同じになり、余分な時間もかからない。得られた問題では既定の形の分の時間が余分にかかる
    if clqprb && clqprb_has('g') && !CLQPRB_FORCE.with(|f| f.get()) && !CLQPRB_STICKY.with(|f| f.get()) {
        let saved = prob.clone();
        let flags = (PROBE_EQUIV_FORCE.with(|f| f.get()), EQUIV2_USED.with(|f| f.get()));
        let tg = std::time::Instant::now();
        CLQPRB_SUPPRESS.with(|f| f.set(true));
        PROBE_GAIN.with(|f| f.set((0, 0, 0)));
        let r = probe(prob, verbose, submip);
        CLQPRB_SUPPRESS.with(|f| f.set(false));
        let (gf, gt, ge) = PROBE_GAIN.with(|f| f.get());
        if r == Some(false) || gf + gt + ge < tunable!("ENOMOTO_T_MIP_CLQPRB_G_MIN", 1usize, usize) {
            return r;
        }
        if verbose {
            eprintln!("MIP: probing: CLQPRB=g: the default probing found {gf} fixed, {gt} tightened, {ge} equivalent pairs ({:.2}s); redoing it with the implications", tg.elapsed().as_secs_f64());
        }
        // `r` (試作): 含意を使う形が既定の形より多く (固定 + 締め付け + 同値な組) 得なければ、既定の形の結果に戻す。
        // neos-860300 (どちらも固定 165)・timtab1 (どちらも同値な組 22)・neos-4722843-widden (含意を使う形は予算を使い切って固定 1) で、
        // 得たものは同じか少ないのに含意の数だけが変わって後の流れが変わり、種をまたいで遅くなっていた
        let revert = clqprb_has('r') || clqprb_has('s');
        let def_state = if revert {
            Some((prob.clone(), PROBE_EQUIV.with(|e| e.borrow().clone()), PROBE_EQUIV_FORCE.with(|f| f.get()), EQUIV2_USED.with(|f| f.get()), r))
        } else {
            None
        };
        *prob = saved;
        PROBE_EQUIV.with(|e| e.borrow_mut().clear());
        PROBE_EQUIV_FORCE.with(|f| f.set(flags.0));
        EQUIV2_USED.with(|f| f.set(flags.1));
        CLQPRB_FORCE.with(|f| f.set(true));
        PROBE_GAIN.with(|f| f.set((0, 0, 0)));
        let r = probe(prob, verbose, submip);
        CLQPRB_FORCE.with(|f| f.set(false));
        let (cf, ct, ce) = PROBE_GAIN.with(|f| f.get());
        if let Some((dp, de, dff, deu, dr)) = def_state {
            // `s` (gate2 候補 2、analysis/gate2_20261117/clqprb_s/README.md): 成果の比 ρ = (含意の形の成果) / (既定の形の成果) が
            // `ENOMOTO_T_MIP_CLQPRB_S_RHO` (既定 2) 未満なら戻す。`r` は ρ ≤ 1 で戻す形。puriri (ρ 1.77) は含意で増えた固定で locks の sub-MIP が
            // 実行不能になり解を失い、mzzv11 (3.84)・neos8 (4.04)・mzzv42z (3.42) は効く。M・P の 106 問で ρ は ≤ 1.77 と ≥ 3.42 に分かれる
            // (シグモイドの形で測ったが、0.5 で切るので同じ働きの if にした)
            let (dsum, csum) = ((gf + gt + ge) as f64, (cf + ct + ce) as f64);
            let keep = if clqprb_has('s') {
                let rho = csum.max(0.5) / dsum.max(0.5);
                if verbose {
                    eprintln!("MIP: probing: CLQPRB=s: gain ratio {rho:.3}");
                }
                rho >= tunable!("ENOMOTO_T_MIP_CLQPRB_S_RHO", 2.0f64, f64)
            } else {
                csum > dsum
            };
            if r != Some(false) && !keep {
                if verbose {
                    eprintln!("MIP: probing: CLQPRB=r: the implications found {cf} fixed, {ct} tightened, {ce} equivalent pairs, not enough over the default; the default result is used");
                }
                *prob = dp;
                PROBE_EQUIV.with(|e| *e.borrow_mut() = de);
                PROBE_EQUIV_FORCE.with(|f| f.set(dff));
                EQUIV2_USED.with(|f| f.set(deu));
                // その求解の後のプロービング (前処理のかけ直し・再スタート) でも判定せずに既定の形を使う
                CLQPRB_REVERTED.with(|f| f.set(true));
                return dr;
            }
        }
        CLQPRB_STICKY.with(|f| f.set(true));
        return r;
    }
    // `ENOMOTO_MIP_CLQPRB` の `n` (試作、analysis/prbimp_20261113.md): 何も固定・締め付けられなければ既定の形でやり直すので、
    // 引き継いだ含意の写しを残しておく
    let clqprb_n = clqprb && clqprb_has('n');
    let edges_in: Vec<(u32, u32)> = if clqprb_n { prob.conflict_edges.clone() } else { Vec::new() };
    let edge_cap = if clqprb {
        (tunable!("ENOMOTO_T_MIP_CLQPRB_EDGE_MULT", 40usize, usize) * nnz).min(tunable!("ENOMOTO_T_MIP_CLQPRB_EDGE_MAX", 6_000_000usize, usize)).max(10 * nnz) + 10_000
    } else {
        tunable!("ENOMOTO_T_MIP_PROBE_EDGE_CAP_MULT", 10usize, usize) * nnz + 10_000
    };
    // 次に重複を除いて詰める本数 (`clqprb` のとき)
    let mut compact_at = 0usize;
    let bin0: Vec<bool> = (0..n).map(|k| prob.is_int[k] && prob.col_lo[k] == 0.0 && prob.col_up[k] == 1.0).collect();
    let genint_on = implbd_genint_for(&prob.is_int, &prob.col_lo, &prob.col_up);
    // 引き継いだ含意 ([`presolve_mip`] の `ENOMOTO_MIP_KEEP_IMPLICATIONS`) に足していく
    let mut edges: Vec<(u32, u32)> = std::mem::take(&mut prob.conflict_edges);
    // 含意による境界 (implied bound カット用): x_j = v で x_k の境界が締まるもの。上限は非零数の 10 倍
    let mut implied: Vec<(u32, bool, u32, bool, f64)> = Vec::new();
    let mut add_implied = |j: usize, v: bool, ch: &[(usize, bool, f64)], dom: &Domain, implied: &mut Vec<(u32, bool, u32, bool, f64)>| {
        let mut best: std::collections::HashMap<(usize, bool), f64> = std::collections::HashMap::new();
        for &(k, upper, val) in ch {
            if k == j {
                continue;
            }
            let e = best.entry((k, upper)).or_insert(val);
            *e = if upper { e.min(val) } else { e.max(val) };
        }
        for ((k, upper), val) in hm_entries(&best) {
            if implied.len() >= edge_cap {
                return;
            }
            // `ENOMOTO_MIP_IMPLBD_GENINT`: 0-1 列どうしの含意は衝突の組 (clique) と同じなので集めない (HiGHS も 0-1 列の含意は
            // clique 表に入れ、implied bound には使わない)。0-1 列の含意で上限を使い切り、一般整数の含意が落ちるのを防ぐ
            if genint_on && bin0[k] {
                continue;
            }
            let (l, u) = (dom.lo[k], dom.up[k]);
            if !l.is_finite() || !u.is_finite() {
                continue;
            }
            let gain = if upper { u - val } else { val - l };
            if gain > 1e-6 * (1.0 + val.abs()) {
                implied.push((j as u32, v, k as u32, upper, val));
            }
        }
    };
    let mut add_edges = |j: usize, v: u32, ch: &[(usize, bool, f64)], edges: &mut Vec<(u32, u32)>| {
        let lj = 2 * j as u32 + if v == 1 { 0 } else { 1 };
        for &(k, upper, val) in ch {
            if k == j || !bin0[k] || edges.len() >= edge_cap {
                continue;
            }
            // x_k <= 0 なら x_k = 1 (文字 2k) と衝突、x_k >= 1 なら x_k = 0 (文字 2k+1) と衝突
            if upper && val <= 0.5 {
                edges.push((lj, 2 * k as u32));
            } else if !upper && val >= 0.5 {
                edges.push((lj, 2 * k as u32 + 1));
            }
        }
    };
    // 成果連動の予算 (既定。`ENOMOTO_MIP_NO_PROBE_ADAPTIVE` で旧来の上限に戻す。サブ MIP 以外): HiGHS の `runProbing` と同じく、試す回数の予算を成果で伸ばす。
    // 予算は候補の数から始め、1 回の試行で固定・締め付けた数だけ増やす。候補を一巡して
    // 固定が候補の 1% 以上出ていれば、残った候補でもう一巡する (最大 `ENOMOTO_T_MIP_PROBE_PASSES` 巡、既定 10。
    // HiGHS は前処理のループで probing を繰り返す)。手間 (`ENOMOTO_T_MIP_PROBE_ADAPT_WORK` 倍、既定 2000 倍) と時間
    // (`ENOMOTO_T_MIP_PROBE_ADAPT_TIME` 秒、既定 3 秒) は安全網。30n20b8 は既定の上限 (非零数の 20 倍) では 7971 本中 160 本で
    // 止まり 1 本も固定できないが、HiGHS は probing を 7 巡 (約 7 秒) 回して列を 8033 -> 4733 本にする
    // サブ MIP でも使うのは `ENOMOTO_MIP_PROBE_ADAPTIVE_SUBMIP` のとき (時間は `ENOMOTO_T_MIP_PROBE_ADAPT_TIME_SUB` 秒、既定 0.5 秒。
    // HiGHS はサブ MIP の前処理でも probing を回す)
    // 40 問・300 秒・4 回 (旧来 / 成果連動): 平均時間 65.71/66.13/67.21/62.22 と 66.33/66.94/66.80/63.82、解けた数
    // 23/22/22/23 と 23/23/23/23 (pk1 が 4 回とも解ける。旧来は 2 回)、主ギャップの和は 2.89/3.03/3.10/2.83 と
    // 2.39/3.02/2.63/2.76。neos-860300 は 31-50 秒 -> 45-49 秒と遅くなる
    let adaptive = env_str!("ENOMOTO_MIP_NO_PROBE_ADAPTIVE").is_none() && (!submip || env_str!("ENOMOTO_MIP_PROBE_ADAPTIVE_SUBMIP").is_some());
    let (work_cap, time_cap, time_cap_found) = if adaptive {
        let t = if submip {
            tunable!("ENOMOTO_T_MIP_PROBE_ADAPT_TIME_SUB", 0.5, f64)
        } else if clqprb {
            // `ENOMOTO_MIP_CLQPRB`: 固定・締め付けが出ているときの時間の上限 (既定 6 秒。HiGHS は mzzv11 の前処理に 30 秒かける)
            tunable!("ENOMOTO_T_MIP_CLQPRB_TIME", 6.0, f64)
        } else {
            tunable!("ENOMOTO_T_MIP_PROBE_ADAPT_TIME", 3.0, f64)
        };
        // 何も固定・締め付けられない間は `ENOMOTO_T_MIP_PROBE_ADAPT_TIME0` 秒 (既定 1 秒) で止める。3 秒のままだと長い行の
        // 多い問題で何も得られずに使い切る (40 問・3 回で air03 0.9 -> 4.0 秒、nw04 9.3 -> 11.9 秒)。30n20b8 は最初の固定まで
        // 約 0.6 秒
        let t0 = tunable!("ENOMOTO_T_MIP_PROBE_ADAPT_TIME0", 1.0, f64).min(t);
        ((tunable!("ENOMOTO_T_MIP_PROBE_ADAPT_WORK", 2000.0, f64) * nnz as f64) as u64 + 100_000, t0, t)
    } else {
        (work_cap, time_cap, time_cap_found)
    };
    // `ENOMOTO_MIP_PRESOLVE_WORK` (試作): 上の時間の上限 (CPU 時間) を手間 (伝播で走査した行の長さの合計) の上限に置き換える
    // ([`presolve_work_caps`])。時間の上限は安全網として `ENOMOTO_T_MIP_PRESOLVE_WORK_SAFE` 倍に延ばす
    let (wcap0, wcap1) = if presolve_work() {
        if adaptive && !submip {
            (tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE0", 1.15e9, f64) as u64, tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE", 3.0e9, f64) as u64)
        } else {
            let w = tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE_SUB", 5.0e8, f64) as u64;
            (w, w)
        }
    } else {
        (u64::MAX, u64::MAX)
    };
    let (time_cap, time_cap_found) = if presolve_work() { (time_cap * presolve_work_safe(), time_cap_found * presolve_work_safe()) } else { (time_cap, time_cap_found) };
    // `ENOMOTO_MIP_PRBWORK` ([`prbwork_on`]): 成果 0 のまま上限で打ち切ったか
    let prbwork = prbwork_active(submip);
    let mut cut_dry = false;
    // 手間 (およそナノ秒): 2.5 x 伝播で走査した行の長さ + 5 x 境界の変更で更新した活動の数 + 68 x 境界の変更の回数 + 190 x 集めた含意の数
    // (+ 1 回の試しごとに `ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE_EACH`、既定 0)。重みは 15 問の CPU 時間への回帰
    // (analysis/presolve_work/fitprobe.py)。行の長さだけでは 1 秒あたり 3.4e6〜4e8 と散らばった
    let act0 = domain::act_work();
    let chg0 = domain::chg_count();
    let e_init = edges.len();
    let each = tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE_EACH", 0.0, f64);
    let (ww, wa, wc, wi) = (
        tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE_W_ROW", 2.5, f64),
        tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE_W_ACT", 5.0, f64),
        tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE_W_CHG", 68.0, f64),
        tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE_W_IMPL", 190.0, f64),
    );
    // `ENOMOTO_MIP_PREPAR` の `p` ([`prepar_has`]): ほかのスレッドで試した分の手間 (行の走査, 活動の更新, 境界の変更)。
    // 手間はスレッドごとに数えるので、使った試しの分をここに足して、逐次と同じ数え方にする
    let spec_w: std::cell::Cell<(u64, u64, u64)> = std::cell::Cell::new((0, 0, 0));
    let pwork = |dom: &Domain, probed: usize, n_edges: usize| {
        let sw = spec_w.get();
        (ww * (dom.debug_work() - work0 + sw.0) as f64 + wa * (domain::act_work() - act0 + sw.1) as f64 + wc * (domain::chg_count() - chg0 + sw.2) as f64 + wi * n_edges.saturating_sub(e_init) as f64 + each * probed as f64) as u64
    };
    // 予算の初期値は候補の数 (全部を一巡できる)。`ENOMOTO_T_MIP_PROBE_BASE` > 0 ならその回数。HiGHS の 1000 回は含意の多い列
    // から試す順番と組み合わせたもので、行の多い列から試すこちらの順番では、30n20b8 の固定できる列 (3 行の短い列、3161 本)
    // が候補の後ろ (4730 番目以降) に並び、1000 回では 1 本も固定できない (全部を試しても 0.96 秒で 3305 本固定)
    let base = tunable!("ENOMOTO_T_MIP_PROBE_BASE", 0.0, f64);
    // `ENOMOTO_MIP_CLQPRB`: 予算の初期値を候補の数の `ENOMOTO_T_MIP_CLQPRB_BASE_MULT` 倍 (既定 2) にする (含意を使う伝播で 2 巡目にも固定が出る)
    let base_mult = if clqprb { tunable!("ENOMOTO_T_MIP_CLQPRB_BASE_MULT", 2.0, f64) } else { 1.0 };
    let mut contingent = if adaptive { if base > 0.0 { base } else { base_mult * cands.len() as f64 } } else { f64::INFINITY };
    let max_pass = if adaptive { tunable!("ENOMOTO_T_MIP_PROBE_PASSES", 10usize, usize).max(1) } else { 1 };
    let want_equiv = env_str!("ENOMOTO_MIP_NO_PROBE_EQUIV").is_none() && !submip;
    // `ENOMOTO_MIP_PROBE_CLIQUE`: 見つけた含意を後の列のプロービングの伝播に使う ([`domain::Domain::add_implications`])
    // `ENOMOTO_MIP_PROBE_CLIQUE=equiv`: 含意は同値な列を見つけるためだけに使う (下の 2 巡目。固定・締め付け・含意の表は変えない)。
    // `ENOMOTO_MIP_PROBE_EQUIV2` (既定 1、`0` で外す。計測 23 で既定化): 同じ 2 巡目を回し、見つけた組が候補の `ENOMOTO_T_MIP_PROBE_EQUIV2_MIN_FRAC` (既定 5%) 以上のときだけ使い、
    // そのときは境界を締めていても代入まで進む ([`probe_equiv_fix`])。少なければ捨てる (既定と同じ問題になる)
    // 再スタート後の問題では既定では使わない (`ENOMOTO_MIP_PROBE_EQUIV2_RESTART` で使う。束の候補、analysis/eqrst_20261113.md)。
    // 以前は uccase12 の再スタート後の問題で組を代入した問題の根の LP が 12043.6 と最適値 11507.405 を越え、誤った最適値 11507.90 を
    // 返したので外した。原因は組ではなく、組の後の前処理の係数の強化が、幅 1e-9 の連続列を持つ許容誤差の範囲で冗長な行を縮めて
    // 最適解を切ったこと (analysis/equiv2_bug_20261022.md)。係数の強化の防御 ([`tighten_coefficients`]) と幅の小さな列の固定
    // ([`fix_tiny_width_columns`]) で直っていて、`ENOMOTO_MIP_PROBE_EQUIV2_RESTART` で再スタート後に組を使う乱数の問題・uccase12・
    // MIPLIB 58 問で誤りは出ない。既定にしないのは計測 (58 問 × 2 種以上) を通していないため
    let equiv2 = env_def!("ENOMOTO_MIP_PROBE_EQUIV2", "1").is_some() && !submip && !(RESTART_PRESOLVE.with(|f| f.get()) && env_str!("ENOMOTO_MIP_PROBE_EQUIV2_RESTART").is_none());
    let pclique_equiv = (equiv2 || env_str!("ENOMOTO_MIP_PROBE_CLIQUE").is_some_and(|v| v == "equiv")) && !submip;
    // 試作 `ENOMOTO_MIP_GAP4` の `q` (既定では使わない。analysis/gap4_rootstuck_mem_20261109.md): `PROBE_CLIQUE` と同じく見つけた含意を
    // 後の列のプロービングの伝播に使う (EQUIV2 の 2 巡目は残す)。HiGHS の probing はクリークの表で伝播するので、neos-3216931-puriri では
    // ENOMOTO の前処理の後でもさらに 2 値列を 203 本固定し、根の LP が 58940 → 60272 (HiGHS) と開いていた。`q` で根の LP 60237
    // `ENOMOTO_MIP_CLQPRB` も同じく含意を伝播に使う
    let prbt = prb_table('a') && !submip;
    let want_pclique = ((env_str!("ENOMOTO_MIP_PROBE_CLIQUE").is_some() && !pclique_equiv) || solver::gap4('q') || clqprb || prbt) && !submip;
    // 試作 `ENOMOTO_MIP_PRB_TABLE` の `a` ([`prb_table`]): 衝突の表 (文字の組、小さい方を上位 32 ビットに)。行から作る衝突と、引き継いだ含意で始める
    let tkey = |a: u32, b: u32| -> u64 { ((a.min(b) as u64) << 32) | a.max(b) as u64 };
    let mut tset: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let (mut t_fix, mut t_sub) = (0usize, 0usize);
    if prbt {
        let g = clique::CliqueGraph::build(n, &prob.rows, &prob.row_lo, &prob.row_up, &bin0, &prob.col_lo, &prob.col_up);
        for u in 0..2 * n as u32 {
            for &v in g.neighbors(u) {
                if v > u {
                    tset.insert(tkey(u, v));
                }
            }
        }
        for &(a, b) in &edges {
            tset.insert(tkey(a, b));
        }
    }
    let mut equiv: Vec<(usize, usize, bool)> = Vec::new();
    // `ENOMOTO_MIP_PROBE_STRENGTHEN` (試作、既定では使わない。サブ MIP では使わない): プロービングによる行の係数の強化
    // ([`probe_strengthen_collect`])。CBC の前処理 (CglPreProcess が CglProbing を `setRowCuts(3)` で使い、`strengthenRow`
    // に行の置き換えを集める。CglPreProcess.cpp 110-130、CglProbing.cpp 5893-6060・6677-6772) にならう。候補は行ごとに
    // 新しい行の係数の絶対値の和が最小の 1 本だけ残し (CglProbing と同じ)、プロービングの後でまとめて行を置き換える
    let want_strengthen = env_str!("ENOMOTO_MIP_PROBE_STRENGTHEN").is_some() && !submip;
    let mut st_mark: Vec<bool> = if want_strengthen { vec![false; prob.m] } else { Vec::new() };
    let mut st_touched: Vec<usize> = Vec::new();
    // x_j = 0 と x_j = 1 で見つけた候補 (行, 列 j の係数の増分, 下限の増分, 上限の増分)
    let (mut st_c0, mut st_c1): (Vec<(usize, f64, f64, f64)>, Vec<(usize, f64, f64, f64)>) = (Vec::new(), Vec::new());
    // 行 -> (新しい行の係数の絶対値の和, 列 j, 係数の増分, 下限の増分, 上限の増分)
    let mut st_best: std::collections::HashMap<usize, (f64, usize, f64, f64, f64)> = std::collections::HashMap::new();
    let mut passes = 0usize;
    // `ENOMOTO_MIP_PREPAR` の `p` (試作、既定では使わない。analysis/prepar_20261107.md): 先の候補をほかのスレッドで投機的に試す
    // ([`ProbeSpec`])。候補の並びの窓 (`PROBE_PAR_BLOCKS` 個の塊、塊は `bsz` 個の候補) を、窓を作った時点の定義域の写しで塊ごとに
    // 逐次に試す。今の定義域が窓を作った時点から変わっていなければ (固定・締め付けがなければ) 結果をそのまま使い、変わっていれば
    // 次の候補から窓を作り直す。窓・塊の大きさはスレッドの数によらず決まるので、結果はスレッドの数・順序によらない
    let par_threads = PREPAR_THREADS.with(|t| t.get());
    // 試作 `ENOMOTO_MIP_PROBE_PAR` (analysis/rootpar_20261122 の 3 節、既定では使わない): 含意を伝播に使う形 (`want_pclique`、`ENOMOTO_MIP_CLQPRB` の
    // やり直しなど) でも窓を使う。窓の塊は窓を作った時点の定義域 (含意の表を含む) の写しで試すので、同じ窓の前の候補が足した含意は使わない
    // (逐次より弱いが、含意はどれも大域的に成り立つので正しい)。窓・塊の大きさは数えた量だけで決まるので、結果はスレッドの数・順序によらない。
    // 1 スレッドの動きは変えない (2 スレッド以上のときだけ)。`ENOMOTO_MIP_PRB_TABLE` の `a` (`prbt`) とは使わない
    let probe_par = env_str!("ENOMOTO_MIP_PROBE_PAR").is_some() && par_threads > 1 && !prbt;
    let par = prepar_has('p') && !submip && !want_strengthen && (!want_pclique || probe_par) && par_threads >= 1 && !solver::in_submip_thread();
    let pool = par.then(|| rayon::ThreadPoolBuilder::new().num_threads(par_threads).build().ok()).flatten();
    let par_blocks = tunable!("ENOMOTO_T_MIP_PREPAR_BLOCKS", 4usize, usize).max(1);
    let (bsz_min, bsz_max) = (2usize, tunable!("ENOMOTO_T_MIP_PREPAR_BSZ_MAX", 32usize, usize).max(2));
    let mut bsz = tunable!("ENOMOTO_T_MIP_PREPAR_BSZ", 8usize, usize).clamp(bsz_min, bsz_max);
    // 今の定義域の版 (固定・締め付けのたびに増やす)
    let mut ver = 0u64;
    let (mut spec, mut spec_start, mut spec_ver, mut spec_used): (Vec<Option<ProbeSpec>>, usize, u64, usize) = (Vec::new(), 0, u64::MAX, 0);
    let (mut n_windows, mut n_spec_used) = (0usize, 0usize);
    // 逐次に試す残りの候補の数と、次に逐次にするときの数 (固定が続くとき)。試し 1 回の手間の和と数 (塊の大きさの下限に使う)
    let (mut seq_left, mut seq_run) = (0usize, 32usize);
    let (mut sum_units, mut n_units) = (0.0f64, 0usize);
    let clone_mult = tunable!("ENOMOTO_T_MIP_PREPAR_CLONE_MULT", 0.25, f64);
    // `ENOMOTO_MIP_ENUM_WHERE=before` (試作 [`enum_where`]): 列挙の前処理をプロービングの前に回す
    if enum_presolve_mode().is_some() && !submip && enum_where() == "before" {
        match enum_presolve_step(prob, &mut dom, &bin0, want_equiv, &mut equiv, &mut edges, verbose, "before probing") {
            None => return Some(false),
            Some((f, t)) => {
                fixed += f;
                tightened += t;
            }
        }
    }
    'passes: for _pass in 0..max_pass {
        passes += 1;
        // `ENOMOTO_MIP_CLQPRB`: 2 巡目からは、含意の重複を除き、伝播に使う含意の表を作り直す (同じ含意を 2 度集めて表が膨らむのを防ぐ)。
        // 1 巡目でも引き継いだ含意 (前処理のかけ直しの前のプロービングの結果) があれば表に入れる
        if clqprb && want_pclique && (passes > 1 || !edges.is_empty()) {
            edges.sort_unstable();
            edges.dedup();
            dom.reset_implications(prob, &edges);
            compact_at = edges.len() + edge_cap / 8;
            if !dom.propagate(prob) {
                return Some(false);
            }
        }
        let fixed_pass0 = fixed;
        spec_ver = u64::MAX;
        // 試作 `ENOMOTO_MIP_PROBE_USELESS` = K (既定では使わない。analysis/scipslow_20261119 の 3・5 節): SCIP のプロービングの打ち切り
        // (`maxtotaluseless` 50・`maxuseless` 1000) と同じ形。固定も締め付けも増えず、新しい含意 (試した列と同じ詰め込みの行に入らない列への含意。同じ
        // 詰め込みの行に入る列への含意はクリーク表にあるので数えない) も無い試しが K 回続いたら、プロービングをやめる (固定も締め付けも増えない試しの回数の上限
        // `ENOMOTO_T_MIP_PROBE_USELESS_MAX` (SCIP の `maxuseless` 1000 に当たる) は既定で使わない: 1000 にすると physiciansched6-2 が 37 → 80 秒。
        // 新しい含意が出続けていた)。air03 は 512 列を試して固定・締め付け・同値な組が 0 のまま 2.9 秒 (含意は行から出る 92 万本。
        // SCIP は 51 回で打ち切り 0.4 秒)。最初の版 (含意を数えず K 回) は drayage-25-23 で含意 5 万本を失い全種で 3.4 倍遅かった
        let useless_k = env_str!("ENOMOTO_MIP_PROBE_USELESS").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
        let useless_max = tunable!("ENOMOTO_T_MIP_PROBE_USELESS_MAX", usize::MAX, usize);
        let (mut useless_run, mut total_run, mut useless_last) = (0usize, 0usize, fixed + tightened);
        let mut prev_probe: Option<(usize, usize)> = None;
        let mut row_mark: Vec<u32> = if useless_k > 0 { vec![u32::MAX; prob.n] } else { Vec::new() };
        for (cnt, &j) in cands.iter().enumerate() {
            if useless_k > 0 {
                // 直前の試しの結果を数える
                if let Some((pj, e0)) = prev_probe.take() {
                    if fixed + tightened > useless_last {
                        useless_run = 0;
                        total_run = 0;
                        useless_last = fixed + tightened;
                    } else {
                        useless_run += 1;
                        let tag = pj as u32;
                        for &(i, _) in &prob.cols[pj] {
                            // 詰め込みの行 (0-1 の列だけで係数が全て 1、上限 1) だけを見る。ほかの行 (ナップサック・変数上下限など) から出る含意は
                            // クリーク表に無いので新しい含意として数える (neos-827175 は詰め込みでない行の含意 59 万本を失い、根で閉じなくなった)
                            if !(prob.row_up[i] <= 1.0 + 1e-9 && prob.rows[i].iter().all(|&(k, a)| a == 1.0 && prob.is_int[k] && prob.col_lo[k] >= 0.0 && prob.col_up[k] <= 1.0)) {
                                continue;
                            }
                            for &(k, _) in &prob.rows[i] {
                                row_mark[k] = tag;
                            }
                        }
                        let fresh = edges.len() > e0 && edges[e0..].iter().any(|&(_, b)| row_mark[(b / 2) as usize] != tag);
                        total_run = if fresh { 0 } else { total_run + 1 };
                    }
                }
                if total_run >= useless_k || useless_run >= useless_max {
                    if verbose {
                        eprintln!("MIP: probing: PROBE_USELESS: stopped after {total_run} successive probes with nothing new ({useless_run} without fixings or tightenings)");
                    }
                    break 'passes;
                }
            }
            let (cap, wcap) = if fixed + tightened > 0 { (time_cap_found, wcap1) } else { (time_cap, wcap0) };
            // 診断用 (`ENOMOTO_MIP_DEBUG_PROBE_PROGRESS`): 16 候補ごとの経過 (CPU 時間・試した数・固定・締め付け・含意の数)
            if cnt % 16 == 0 && verbose && env_str!("ENOMOTO_MIP_DEBUG_PROBE_PROGRESS").is_some() {
                eprintln!("MIP:   probing progress: candidate {cnt} (pass {passes}), probed {probed}, fixed {fixed}, tightened {tightened}, implications {}, cpu {:.3}s", edges.len(), cap_clock() - c0);
            }
            if (cnt % 16 == 0 && (dom.debug_work() - work0 + spec_w.get().0 > work_cap || pwork(&dom, probed, edges.len()) > wcap || cap_clock() - c0 > cap || presolve_stop())) || (presolve_stop_fine() && presolve_stop()) {
                cut_dry = fixed + tightened == 0 && !presolve_stop();
                break 'passes;
            }
            if dom.lo[j] == dom.up[j] {
                continue;
            }
            if probed as f64 >= contingent {
                break 'passes;
            }
            let found0 = fixed + tightened;
            probed += 1;
            if useless_k > 0 {
                prev_probe = Some((j, edges.len()));
            }
            // `ENOMOTO_MIP_PREPAR` の `p`: 窓の結果を使う (窓がない・古ければ、ここから窓を作る)
            let spec_res = if let Some(pool) = &pool {
                let valid = spec_ver == ver && cnt >= spec_start && cnt < spec_start + spec.len();
                if !valid && !spec.is_empty() {
                    // 前の窓の使われ方で塊の大きさを決める (使い切れば倍、1/4 未満で 1/4。候補の並びだけで決まる)。
                    // 塊が最小でも 1/4 未満しか使えなければ (固定が続く)、次の `seq_run` 個の候補は逐次に試す (続けば倍)
                    if spec_used * 4 < spec.len() {
                        if bsz == bsz_min {
                            seq_left = seq_run;
                            seq_run = (seq_run * 2).min(1024);
                        }
                        bsz = (bsz / 4).max(bsz_min);
                    } else if spec_used >= spec.len() {
                        bsz = (bsz * 2).min(bsz_max);
                        seq_run = 32;
                    }
                    spec.clear();
                }
                if valid {
                    // 窓の結果を使う
                } else if seq_left > 0 {
                    seq_left -= 1;
                } else {
                    // 塊の手間が写しの手間の `ENOMOTO_T_MIP_PREPAR_CLONE_MULT` 倍 (既定 0.25。neos-4722843-widden で写し 1 回 2.4 ミリ秒、試し 1 回 約 1 ミリ秒。4 倍だと塊が 300 を越えて、締め付けで崩れた窓の試しが無駄になった) 以上になるよう塊を大きくする
                    // (1 回の試しが軽い問題で、定義域の写しが試しより重くなるのを防ぐ。手間は数えた量なので決定的)。
                    // 手間の重みは pwork と同じ (行の走査 2.5、活動の更新 5)、写しは 1 要素 16 とみる
                    let lo = if n_units > 0 {
                        let per = (sum_units / n_units as f64).max(1.0);
                        ((clone_mult * 16.0 * (prob.n + prob.m) as f64 / per).ceil() as usize).clamp(bsz_min, tunable!("ENOMOTO_T_MIP_PREPAR_BSZ_WORK_MAX", 64usize, usize))
                    } else {
                        bsz_min
                    };
                    let b = bsz.max(lo);
                    let end = (cnt + b * par_blocks).min(cands.len());
                    let base: &Domain = &dom;
                    let pr: &MipProblem = prob;
                    // 窓が候補の残りで短くなったときも塊の数がそろうよう、窓を塊の数で等分する
                    let b = (end - cnt).div_ceil(par_blocks).max(1);
                    // 塊 k は窓の k, k + 塊の数, k + 2 x 塊の数, ... 番目の候補 (候補は行の多い列からなので、続きで分けると前の塊に重い試しが集まる)
                    let nb = (end - cnt).min(par_blocks);
                    let blocks_v: Vec<Vec<usize>> = (0..nb).map(|k| cands[cnt..end].iter().skip(k).step_by(nb).copied().collect()).collect();
                    let blocks: Vec<&[usize]> = blocks_v.iter().map(|v| v.as_slice()).collect();
                    let cx = ProbeSpecCtx { bin0: &bin0, genint_on, want_impl, want_implied, want_equiv };
                    let tw = std::time::Instant::now();
                    let sc = PresolveStopCtx::capture();
                    let res: Vec<Vec<Option<ProbeSpec>>> = pool.install(|| {
                        use rayon::prelude::*;
                        blocks.par_iter().map(|bl| PresolveStopCtx::run(&sc, || probe_spec_block(pr, base, bl, &cx))).collect()
                    });
                    // 診断 `ENOMOTO_MIP_PREPAR_DEBUG`: 窓ごとの大きさと時間
                    if env_str!("ENOMOTO_MIP_PREPAR_DEBUG").is_some() {
                        eprintln!("PREPAR window at {cnt}: {} blocks of {b}, {:.4}s", blocks.len(), tw.elapsed().as_secs_f64());
                    }
                    // 窓の順に並べ直す
                    let mut its: Vec<std::vec::IntoIter<Option<ProbeSpec>>> = res.into_iter().map(|v| v.into_iter()).collect();
                    spec = (0..end - cnt).map(|i| its[i % nb].next().flatten()).collect();
                    spec_start = cnt;
                    spec_ver = ver;
                    spec_used = 0;
                    n_windows += 1;
                }
                if !spec.is_empty() {
                    spec_used = cnt - spec_start + 1;
                    spec[cnt - spec_start].take()
                } else {
                    None
                }
            } else {
                None
            };
            let mut derived: Option<ProbeDerived> = None;
            let (ok0, ch0, ok1, ch1) = if let Some(r) = spec_res {
                n_spec_used += 1;
                let w = spec_w.get();
                spec_w.set((w.0 + r.w.0, w.1 + r.w.1, w.2 + r.w.2));
                sum_units += 2.5 * r.w.0 as f64 + 5.0 * r.w.1 as f64;
                n_units += 1;
                derived = r.derived;
                (r.ok0, r.ch0, r.ok1, r.ch1)
            } else {
                let u0 = (dom.debug_work(), domain::act_work());
                let pos = dom.stack_len();
                // x_j = 0 (変わった境界だけ覚える: 全列の境界を写すと列の多い問題で重い)
                dom.tighten_upper(prob, j, 0.0);
                let ok0 = dom.propagate(prob);
                let ch0: Vec<_> = if ok0 { dom.changes_since(pos).collect() } else { Vec::new() };
                st_c0.clear();
                if want_strengthen && ok0 {
                    probe_strengthen_collect(prob, &dom.lo, &dom.up, j, false, &ch0, &mut st_mark, &mut st_touched, &mut st_c0);
                }
                dom.backtrack_to(prob, pos);
                // x_j = 1
                dom.tighten_lower(prob, j, 1.0);
                let ok1 = dom.propagate(prob);
                let ch1: Vec<_> = if ok1 { dom.changes_since(pos).collect() } else { Vec::new() };
                st_c1.clear();
                if want_strengthen && ok1 {
                    probe_strengthen_collect(prob, &dom.lo, &dom.up, j, true, &ch1, &mut st_mark, &mut st_touched, &mut st_c1);
                }
                dom.backtrack_to(prob, pos);
                if pool.is_some() {
                    sum_units += 2.5 * (dom.debug_work() - u0.0) as f64 + 5.0 * (domain::act_work() - u0.1) as f64;
                    n_units += 1;
                }
                (ok0, ch0, ok1, ch1)
            };
            match (ok0, ok1) {
                (false, false) => return Some(false),
                (false, true) | (true, false) => {
                    let v = if ok1 { 1.0 } else { 0.0 };
                    dom.tighten_lower(prob, j, v);
                    dom.tighten_upper(prob, j, v);
                    if !dom.propagate(prob) {
                        return Some(false);
                    }
                    fixed += 1;
                    ver += 1;
                    if env_str!("ENOMOTO_MIP_DEBUG_PROBE").is_some() {
                        eprintln!("MIP:   probing fixed column {j} at candidate {cnt} (pass {passes}, {} rows)", prob.cols[j].len());
                    }
                }
                (true, true) if derived.is_some() => {
                    // `ENOMOTO_MIP_PREPAR` の `p`: 塊のスレッドで集計した分をそのまま足す (定義域は窓の時点と同じ)
                    let dv = derived.take().unwrap();
                    if want_impl {
                        // `ENOMOTO_MIP_PROBE_PAR`: 逐次の道と同じく、上限に近づいたら重複を除いて詰め、足した含意を伝播に使う
                        if clqprb && want_pclique && edges.len() + dv.edges.len() >= edge_cap && edges.len() >= compact_at {
                            edges.sort_unstable();
                            edges.dedup();
                            compact_at = edges.len() + edge_cap / 8;
                        }
                        let e0 = edges.len();
                        for e in dv.edges {
                            if edges.len() < edge_cap {
                                edges.push(e);
                            }
                        }
                        if want_pclique {
                            dom.add_implications(n, &edges[e0..]);
                        }
                        if want_implied {
                            for t in dv.implied {
                                if implied.len() >= edge_cap {
                                    break;
                                }
                                implied.push(t);
                            }
                        }
                    }
                    if want_equiv {
                        equiv.extend(dv.equiv);
                    }
                    let mut any = false;
                    for (k, upper, v) in dv.tight {
                        if upper {
                            if v < dom.up[k] - 1e-9 * (1.0 + v.abs()) {
                                dom.tighten_upper(prob, k, v);
                                any = true;
                                tightened += 1;
                            }
                        } else if v > dom.lo[k] + 1e-9 * (1.0 + v.abs()) {
                            dom.tighten_lower(prob, k, v);
                            any = true;
                            tightened += 1;
                        }
                    }
                    if any {
                        ver += 1;
                        if !dom.propagate(prob) {
                            return Some(false);
                        }
                    }
                }
                (true, true) => {
                    for &(i, dc, dl, du) in st_c0.iter().chain(&st_c1) {
                        // 新しい行の係数の絶対値の和 (CglProbing と同じく小さいものを残す)
                        let aj = prob.rows[i].binary_search_by_key(&j, |t| t.0).map_or(0.0, |q| prob.rows[i][q].1);
                        let score = prob.rows[i].iter().map(|t| t.1.abs()).sum::<f64>() - aj.abs() + (aj + dc).abs();
                        let e = st_best.entry(i).or_insert((f64::INFINITY, 0, 0.0, 0.0, 0.0));
                        if score < e.0 {
                            *e = (score, j, dc, dl, du);
                        }
                    }
                    if want_impl {
                        // `ENOMOTO_MIP_CLQPRB`: 上限に近づいたら重複を除いて詰める (詰めても 1/8 しか空かなければ、もう詰めない)
                        if clqprb && edges.len() + ch0.len() + ch1.len() >= edge_cap && edges.len() >= compact_at {
                            edges.sort_unstable();
                            edges.dedup();
                            compact_at = edges.len() + edge_cap / 8;
                        }
                        let e0 = edges.len();
                        add_edges(j, 0, &ch0, &mut edges);
                        add_edges(j, 1, &ch1, &mut edges);
                        if want_pclique {
                            dom.add_implications(n, &edges[e0..]);
                        }
                        // 試作 `ENOMOTO_MIP_PRB_TABLE` の `a`: 新しい衝突ごとに HiGHS の `processNewEdge` の規則 (固定・同値な組)
                        if prbt {
                            let mut tfix: Vec<u32> = Vec::new();
                            for &(a, b) in &edges[e0..] {
                                if a / 2 == b / 2 {
                                    if a == b {
                                        tfix.push(a);
                                    }
                                    continue;
                                }
                                if tset.contains(&tkey(a ^ 1, b)) {
                                    tfix.push(b);
                                } else if tset.contains(&tkey(b ^ 1, a)) {
                                    tfix.push(a);
                                } else if want_equiv && tset.contains(&tkey(a ^ 1, b ^ 1)) {
                                    // a・b のちょうど一方が真: 文字の向きが同じなら x_b = 1 − x_a、違えば x_b = x_a
                                    equiv.push(((a / 2) as usize, (b / 2) as usize, (a & 1) == (b & 1)));
                                    t_sub += 1;
                                }
                                tset.insert(tkey(a, b));
                            }
                            let mut anyfix = false;
                            for l in tfix {
                                let k = (l / 2) as usize;
                                // 文字 l は偽: l = 2k (x_k = 1) なら x_k = 0、l = 2k + 1 (x_k = 0) なら x_k = 1
                                let v = if l & 1 == 0 { 0.0 } else { 1.0 };
                                if dom.lo[k] == dom.up[k] {
                                    continue;
                                }
                                dom.tighten_lower(prob, k, v);
                                dom.tighten_upper(prob, k, v);
                                fixed += 1;
                                t_fix += 1;
                                anyfix = true;
                            }
                            if anyfix {
                                ver += 1;
                                if !dom.propagate(prob) {
                                    return Some(false);
                                }
                            }
                        }
                        // implied bound カットを使うときだけ集める (既定は相手が連続変数のものだけ使う。`ENOMOTO_MIP_NO_IMPLBD_CONT` で使わない)
                        if env_str!("ENOMOTO_MIP_IMPLBD_CUTS").is_some() || env_str!("ENOMOTO_MIP_NO_IMPLBD_CONT").is_none() {
                            add_implied(j, false, &ch0, &dom, &mut implied);
                            add_implied(j, true, &ch1, &dom, &mut implied);
                        }
                    }
                    // 両方の結果で変わった (列, 側) だけが締まりうる。値は両方の弱い方
                    let mut first: std::collections::HashMap<(usize, bool), f64> = std::collections::HashMap::new();
                    for &(k, upper, v) in &ch0 {
                        first.insert((k, upper), v);
                    }
                    let mut last1: std::collections::HashMap<(usize, bool), f64> = std::collections::HashMap::new();
                    for &(k, upper, v) in &ch1 {
                        last1.insert((k, upper), v);
                    }
                    // 同値な 0-1 列 (既定で集める。`ENOMOTO_MIP_NO_PROBE_EQUIV` で集めない): x_j = 0 で x_k = 0、x_j = 1 で x_k = 1 なら x_k = x_j。
                    // 逆向きなら x_k = 1 - x_j。等式の行として足して前処理をかけ直す ([`presolve_mip`]、HiGHS の probing の代入)
                    if want_equiv {
                        for ((k, upper), v0) in hm_entries(&first) {
                            if k == j || !bin0[k] || dom.lo[k] == dom.up[k] {
                                continue;
                            }
                            let Some(&v1) = last1.get(&(k, !upper)) else { continue };
                            if upper && v0 <= 0.5 && v1 >= 0.5 {
                                equiv.push((j, k, false));
                            } else if !upper && v0 >= 0.5 && v1 <= 0.5 {
                                equiv.push((j, k, true));
                            }
                        }
                    }
                    let mut any = false;
                    for ((k, upper), v0) in hm_entries(&first) {
                        if k == j {
                            continue;
                        }
                        let Some(&v1) = last1.get(&(k, upper)) else { continue };
                        if upper {
                            let u = v0.max(v1);
                            if u < dom.up[k] - 1e-9 * (1.0 + u.abs()) {
                                dom.tighten_upper(prob, k, u);
                                any = true;
                                tightened += 1;
                            }
                        } else {
                            let l = v0.min(v1);
                            if l > dom.lo[k] + 1e-9 * (1.0 + l.abs()) {
                                dom.tighten_lower(prob, k, l);
                                any = true;
                                tightened += 1;
                            }
                        }
                    }
                    if any {
                        ver += 1;
                    }
                    if any && !dom.propagate(prob) {
                        return Some(false);
                    }
                }
            }
            contingent += (fixed + tightened - found0) as f64;
        }
        // もう一巡するのは、この巡で候補の 1% 以上を固定したときだけ
        if ((fixed - fixed_pass0) as f64) < 0.01 * cands.len() as f64 {
            break;
        }
    }
    // `ENOMOTO_MIP_CLQPRB` の `n` (試作、analysis/prbimp_20261113.md): 含意を使っても何も固定・締め付けられなければ (固定・締め付けの数が
    // 候補の `ENOMOTO_T_MIP_CLQPRB_N_FRAC` (既定 0) 以下なら)、集めた含意・同値な組を捨てて既定の形でプロービングをやり直す。
    // roll3000・cbs-cta は含意を使っても何も固定できず、増えた含意 (クリークのグラフ・implied bound) と同値な組 (cbs-cta の 3 組) で後の流れだけが変わって遅くなった。
    // やり直しの分の時間 (何も固定できない間の上限 `ENOMOTO_T_MIP_PROBE_ADAPT_TIME0` 秒まで) が余分にかかる
    if clqprb_n && ((fixed + tightened) as f64) <= tunable!("ENOMOTO_T_MIP_CLQPRB_N_FRAC", 0.0, f64) * cands.len() as f64 {
        if verbose {
            eprintln!(
                "MIP: probing: CLQPRB=n: {probed} probed, {fixed} fixed, {tightened} tightened, {} implications, {} equivalent pairs with implications ({:.2}s); redoing the default probing",
                edges.len(),
                equiv.len(),
                t0.elapsed().as_secs_f64()
            );
        }
        prob.conflict_edges = edges_in;
        CLQPRB_SUPPRESS.with(|f| f.set(true));
        let r = probe(prob, verbose, submip);
        CLQPRB_SUPPRESS.with(|f| f.set(false));
        return r;
    }
    // `ENOMOTO_MIP_ENUM_PRESOLVE` (試作): HiGHS の Enumeration 前処理 ([`enumerate_rows`])。`=impl` ならプロービングの含意も伝播に使う。`=all`・`=allimpl` は行の数・空振りの上限を外す ([`enumerate_rows`])
    // (行の数の上限と空振りでやめる規則を外して全部の候補の行に回す試作 `ENOMOTO_MIP_HO3_ENUM` は 2026-11-18 に削除。
    // analysis/highs_only_20261105/G3.md・analysis/gate2_20261117/ho3sl/README.md)
    if enum_presolve_mode().is_some() && !submip && enum_where() != "before" {
        match enum_presolve_step(prob, &mut dom, &bin0, want_equiv, &mut equiv, &mut edges, verbose, "after probing") {
            None => return Some(false),
            Some((f, t)) => {
                fixed += f;
                tightened += t;
            }
        }
    }
    // `ENOMOTO_MIP_PROBE_CLIQUE=equiv`: 集めた含意をすべて伝播に入れた写しの定義域で 0-1 列をもう一巡試し、同値な組だけを足す。
    // 行の伝播では `x = 1 => y = 0` の対偶が見えないので、同値な組の多くを見落とす (physiciansched6-2 は 8 組、含意を使うと 1084 組)。
    // `PROBE_CLIQUE` そのものは見つけた含意で固定・締め付け・含意の表 (クリークのグラフ) まで変え、ほかの問題の経路も変える
    if want_equiv && pclique_equiv && !edges.is_empty() {
        let tq = std::time::Instant::now();
        let n0 = equiv.len();
        let mut d2 = dom.clone();
        d2.add_implications(n, &edges);
        // 時間の上限 (`ENOMOTO_T_MIP_PROBE_EQUIV2_TIME`、既定 0.5 秒)。`ENOMOTO_T_MIP_PROBE_EQUIV2_GIVEUP_TIME` 秒 (既定 0.1 秒) 経っても
        // 1 組も見つからなければやめる (見つからない問題で 1 秒使い切っていた: nw04・air03・neos-960392)。列の数で打ち切ると
        // (行の多い列から 128 列) physiciansched6-2 の組 (行の少ない列) に届かない
        let time_cap2 = tunable!("ENOMOTO_T_MIP_PROBE_EQUIV2_TIME", 0.5, f64);
        let giveup2 = tunable!("ENOMOTO_T_MIP_PROBE_EQUIV2_GIVEUP_TIME", 0.1, f64);
        // `ENOMOTO_MIP_PRESOLVE_WORK`: 時間の代わりに手間 (0.5 秒・0.1 秒に当たる値) で打ち切る
        let (time_cap2, giveup2, wcap2, wgive2) = if presolve_work() {
            let w = tunable!("ENOMOTO_T_MIP_PRESOLVE_WORK_EQUIV2", 5.0e7, f64) as u64;
            (time_cap2 * presolve_work_safe(), giveup2 * presolve_work_safe(), w, w / 5)
        } else {
            (time_cap2, giveup2, u64::MAX, u64::MAX)
        };
        let w0 = d2.debug_work();
        // `ENOMOTO_MIP_EQUIV2_PWORK` ([`equiv2_pwork`]、既定): `pwork` の重みで数えた手間が `ENOMOTO_T_MIP_EQUIV2_PWORK_CAP` (既定 5e8、0.5 秒相当) を
        // 越えたら止め、`ENOMOTO_T_MIP_EQUIV2_PWORK_GIVEUP` (既定 1e8、0.1 秒相当) を越えても 1 組も見つからなければやめる (時間の上限と同じ値)。
        // 今までの行の走査の上限・時間の安全網もそのまま残す
        let (pw_cap, pw_give) = if presolve_work() && env_def!("ENOMOTO_MIP_EQUIV2_PWORK", "1").is_some() {
            (tunable!("ENOMOTO_T_MIP_EQUIV2_PWORK_CAP", 5.0e8, f64) as u64, tunable!("ENOMOTO_T_MIP_EQUIV2_PWORK_GIVEUP", 1.0e8, f64) as u64)
        } else {
            (u64::MAX, u64::MAX)
        };
        let (a0, c0) = (domain::act_work(), domain::chg_count());
        // `ENOMOTO_MIP_PREPAR` の `e` (試作): この巡は定義域 `d2` を変えない (試して巻き戻すだけ) ので、候補の窓を塊に分けて
        // 塊ごとの `d2` の写しで試し、決まった順にまとめる。手間は写しで数えた分を足して逐次と同じ打ち切りにする。
        // 1 つの候補で見つけた組は列の番号の順に並べる (逐次はハッシュ表の順)
        let epool = (prepar_has('e') && !solver::in_submip_thread()).then(|| rayon::ThreadPoolBuilder::new().num_threads(PREPAR_THREADS.with(|t| t.get()).max(1)).build().ok()).flatten();
        if let Some(pool) = &epool {
            let (eb, ek) = (tunable!("ENOMOTO_T_MIP_PREPAR_EQ_BSZ", 32usize, usize).max(1), tunable!("ENOMOTO_T_MIP_PREPAR_BLOCKS", 4usize, usize).max(1));
            let mut virt = 0u64;
            let mut pvirt = 0u64;
            let mut ci = 0usize;
            'eq: while ci < cands.len() {
                let end = (ci + eb * ek).min(cands.len());
                // 塊 k は窓の k, k + 塊の数, ... 番目の候補 (重い試しが 1 つの塊に集まらないように)
                let nb = (end - ci).min(ek);
                let blocks_v: Vec<Vec<usize>> = (0..nb).map(|k| cands[ci..end].iter().skip(k).step_by(nb).copied().collect()).collect();
                let (base, pr, b0): (&Domain, &MipProblem, &[bool]) = (&d2, prob, &bin0);
                let sc = PresolveStopCtx::capture();
                let res: Vec<Vec<(Vec<(usize, usize, bool)>, u64, u64)>> = pool.install(|| {
                    use rayon::prelude::*;
                    blocks_v.par_iter().map(|bl| PresolveStopCtx::run(&sc, || probe_equiv_block(pr, base, b0, bl))).collect()
                });
                let mut its: Vec<_> = res.into_iter().map(|v| v.into_iter()).collect();
                let ordered: Vec<(Vec<(usize, usize, bool)>, u64, u64)> = (0..end - ci).map(|i| its[i % nb].next().unwrap()).collect();
                for (idx, (pairs, w1, pw1)) in ordered.into_iter().enumerate() {
                    let cj = ci + idx;
                    if virt > work_cap.min(wcap2) || tq.elapsed().as_secs_f64() > time_cap2 || ((tq.elapsed().as_secs_f64() > giveup2 || virt > wgive2) && equiv.len() == n0) || ((cj % 16 == 0 || presolve_stop_fine()) && presolve_stop()) {
                        break 'eq;
                    }
                    if pvirt > pw_cap || (pvirt > pw_give && equiv.len() == n0) {
                        break 'eq;
                    }
                    virt += w1;
                    pvirt += pw1;
                    equiv.extend(pairs);
                }
                ci = end;
            }
        }
        for (ci, &j) in cands.iter().enumerate() {
            if epool.is_some() {
                break;
            }
            // 毎列見る (長い列の多い問題では 16 列で 0.7 秒かかる: nw04)。時間の上限・止めの合図 ([`presolve_stop`]) は 16 列ごと
            let w = d2.debug_work() - w0;
            if w > work_cap.min(wcap2) || tq.elapsed().as_secs_f64() > time_cap2 || ((tq.elapsed().as_secs_f64() > giveup2 || w > wgive2) && equiv.len() == n0) || ((ci % 16 == 0 || presolve_stop_fine()) && presolve_stop()) {
                break;
            }
            if pw_cap < u64::MAX {
                let pw = equiv2_pwork(w, domain::act_work() - a0, domain::chg_count() - c0);
                if pw > pw_cap || (pw > pw_give && equiv.len() == n0) {
                    break;
                }
            }
            if d2.lo[j] == d2.up[j] {
                continue;
            }
            let pos = d2.stack_len();
            d2.tighten_upper(prob, j, 0.0);
            let ok0 = d2.propagate(prob);
            let ch0: std::collections::HashMap<(usize, bool), f64> = if ok0 { d2.changes_since(pos).map(|(k, u, v)| ((k, u), v)).collect() } else { Default::default() };
            d2.backtrack_to(prob, pos);
            d2.tighten_lower(prob, j, 1.0);
            let ok1 = d2.propagate(prob);
            let ch1: std::collections::HashMap<(usize, bool), f64> = if ok1 { d2.changes_since(pos).map(|(k, u, v)| ((k, u), v)).collect() } else { Default::default() };
            d2.backtrack_to(prob, pos);
            if !(ok0 && ok1) {
                continue;
            }
            for ((k, upper), v0) in hm_entries(&ch0) {
                if k == j || !bin0[k] || d2.lo[k] == d2.up[k] {
                    continue;
                }
                let Some(&v1) = ch1.get(&(k, !upper)) else { continue };
                if upper && v0 <= 0.5 && v1 >= 0.5 {
                    equiv.push((j, k, false));
                } else if !upper && v0 >= 0.5 && v1 <= 0.5 {
                    equiv.push((j, k, true));
                }
            }
        }
        let found2 = equiv.len() - n0;
        if equiv2 {
            // 組は両向きで 2 回ずつ数える (j から k、k から j) ので半分で比べる
            if (found2 / 2) as f64 >= tunable!("ENOMOTO_T_MIP_PROBE_EQUIV2_MIN_FRAC", 0.05, f64) * cands.len() as f64 && found2 > 0 {
                PROBE_EQUIV_FORCE.with(|f| f.set(true));
                EQUIV2_USED.with(|f| f.set(true));
            } else {
                equiv.truncate(n0);
            }
        }
        if verbose {
            eprintln!("MIP: probing: equivalence pass with implications: {found2} pairs found ({:.2}s){}", tq.elapsed().as_secs_f64(), if equiv2 { if PROBE_EQUIV_FORCE.with(|f| f.get()) { ", used" } else { ", not used" } } else { "" });
        }
    }
    // `ENOMOTO_MIP_CLQPRB=d` (試作、`clqprb_has('d')`): このプロービングで何も固定・締め付けられなければ、広げた上限で増えた含意を捨てて
    // 既定の上限 (非零数の 10 倍 + 1 万本) に戻す (air03 は何も固定できないのに含意が 92 万 → 267 万本に増え、後段で 1.2〜1.5 倍遅くなった)。
    // 残すのは集めた順の先頭 (重複を除いて詰めた後は列の番号順) で、既定と同じ組とは限らない
    if clqprb && clqprb_has('d') && fixed == 0 && tightened == 0 {
        let cap0 = 10 * nnz + 10_000;
        if edges.len() > cap0 {
            if verbose {
                eprintln!("MIP: probing: nothing fixed; implications {} -> {cap0} (CLQPRB=d)", edges.len());
            }
            edges.truncate(cap0);
        }
    }
    // 固定した列を含む組は捨てる (固定後は意味がない)
    if verbose && env_str!("ENOMOTO_MIP_DEBUG_PROBE_WORKCUT").is_some() {
        eprintln!("MIP: probing: {} implications collected before the dedup (cap {edge_cap})", edges.len());
    }
    edges.retain(|&(a, b)| dom.lo[(a / 2) as usize] < dom.up[(a / 2) as usize] && dom.lo[(b / 2) as usize] < dom.up[(b / 2) as usize]);
    edges.sort_unstable();
    edges.dedup();
    if verbose && env_str!("ENOMOTO_MIP_DEBUG_PROBE_WORKCUT").is_some() {
        eprintln!("MIP: probing: {} propagations stopped by the work limit", domain::work_cuts() - wcut0);
    }
    if verbose && prbt {
        eprintln!("MIP: probing: PRB_TABLE a: {t_fix} columns fixed and {t_sub} equivalent pairs by the conflict table rules ({} table entries)", tset.len());
    }
    if verbose {
        eprintln!(
            "MIP: probing: {probed} of {} binaries probed ({passes} passes), {fixed} fixed, {tightened} bounds tightened, {} implications ({:.2}s, cpu {:.2}s, work {}, pwork {}, chg {})",
            cands.len(),
            edges.len(),
            t0.elapsed().as_secs_f64(),
            cap_clock() - c0,
            dom.debug_work() - work0,
            pwork(&dom, probed, edges.len()),
            domain::chg_count() - chg0
        );
    }
    if verbose && pool.is_some() {
        eprintln!("MIP: probing: PREPAR {par_threads} threads, {n_windows} windows, {n_spec_used} of {probed} probes from the windows");
    }
    PROBE_FIXED.with(|f| f.set(fixed));
    // 診断 `ENOMOTO_MIP_DEBUG_PROBE_SUBST` (振る舞いは変えない、analysis/prbsubst_20261117): HiGHS の probing の代入
    // (`HighsImplications::runProbing`: 0-1 列 x の両側で列 y がそれぞれ違う値に固定されれば y = a + (b − a) x) に当たる組を、
    // プロービングの後の定義域で数える (y の種類別: 0-1・一般の整数・連続)
    if verbose && env_str!("ENOMOTO_MIP_DEBUG_PROBE_SUBST").is_some() {
        let ts = std::time::Instant::now();
        let mut d = dom.clone();
        let (mut nb, mut ni, mut nc, mut nx) = (0usize, 0usize, 0usize, 0usize);
        let mut subst = vec![false; n];
        for &j in &cands {
            if d.lo[j] == d.up[j] || subst[j] {
                continue;
            }
            let pos = d.stack_len();
            d.tighten_upper(prob, j, 0.0);
            let ok0 = d.propagate(prob);
            let (lo0, up0) = (d.lo.clone(), d.up.clone());
            d.backtrack_to(prob, pos);
            d.tighten_lower(prob, j, 1.0);
            let ok1 = d.propagate(prob);
            if ok0 && ok1 {
                for k in 0..n {
                    if k == j || subst[k] || dom.lo[k] == dom.up[k] {
                        continue;
                    }
                    if lo0[k] == up0[k] && d.lo[k] == d.up[k] && (lo0[k] - d.lo[k]).abs() > 1e-6 {
                        subst[k] = true;
                        if bin0[k] {
                            nb += 1;
                        } else if prob.is_int[k] {
                            ni += 1;
                        } else {
                            nc += 1;
                        }
                    }
                }
            } else {
                nx += 1;
            }
            d.backtrack_to(prob, pos);
        }
        eprintln!("MIP: probing: PROBE_SUBST diag: {nb} binary, {ni} general integer, {nc} continuous columns substitutable ({} candidates, {nx} one-sided, {:.2}s)", cands.len(), ts.elapsed().as_secs_f64());
    }
    if !submip && env_str!("ENOMOTO_MIP_DEBUG_IMPLCLASS").is_some() {
        implclass_diag(prob, &edges, e_init, &dom.lo, &dom.up);
    }
    prob.conflict_edges = edges;
    if want_equiv {
        let mut seen = std::collections::HashSet::new();
        equiv.retain(|&(j, k, _)| dom.lo[j] < dom.up[j] && dom.lo[k] < dom.up[k] && seen.insert((j.min(k), j.max(k))));
        if verbose {
            eprintln!("MIP: probing: {} equivalent binary pairs", equiv.len());
        }
        if let Some(id) = &dump_id {
            let txt: String = equiv.iter().map(|&(j, k, neg)| format!("{j} {k} {}\n", neg as u8)).collect();
            let _ = std::fs::write(format!("{id}.pairs"), txt);
        }
        PROBE_EQUIV.with(|e| *e.borrow_mut() = std::mem::take(&mut equiv));
    }
    // 固定した列を含む含意は捨てる
    implied.retain(|&(j, _, k, _, _)| dom.lo[j as usize] < dom.up[j as usize] && dom.lo[k as usize] < dom.up[k as usize]);
    // 診断 `ENOMOTO_MIP_DEBUG_PROBE_IMPLIED` (analysis/prbimp_20261113.md): implied bound の本数
    if verbose && env_str!("ENOMOTO_MIP_DEBUG_PROBE_IMPLIED").is_some() {
        eprintln!("MIP: probing: {} implied bounds kept", implied.len());
    }
    prob.implied_bounds = implied;
    // `ENOMOTO_MIP_PROBE_STRENGTHEN`: 集めた行の置き換えを当てる (列 j が固定された行は飛ばす。固定で行から消える)
    if want_strengthen && !st_best.is_empty() {
        let mut rows_sorted: Vec<(usize, (f64, usize, f64, f64, f64))> = st_best.into_iter().collect();
        rows_sorted.sort_unstable_by_key(|t| t.0);
        let mut k = 0usize;
        for (i, (_, j, dc, dl, du)) in rows_sorted {
            if dom.lo[j] == dom.up[j] {
                continue;
            }
            let row = &mut prob.rows[i];
            match row.binary_search_by_key(&j, |t| t.0) {
                Ok(q) => {
                    let v = row[q].1 + dc;
                    if v.abs() <= 1e-12 {
                        row.remove(q);
                    } else {
                        row[q].1 = v;
                    }
                }
                Err(q) => row.insert(q, (j, dc)),
            }
            prob.row_lo[i] += dl;
            prob.row_up[i] += du;
            k += 1;
        }
        if k > 0 {
            let mut cols: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
            for (i, r) in prob.rows.iter().enumerate() {
                for &(j, v) in r {
                    cols[j].push((i, v));
                }
            }
            prob.cols = cols;
            prob.nnz = prob.rows.iter().map(|r| r.len()).sum();
        }
        if verbose {
            eprintln!("MIP: probing: strengthened {k} rows (PROBE_STRENGTHEN)");
        }
    }
    PROBE_GAIN.with(|f| f.set((fixed, tightened, if want_equiv { PROBE_EQUIV.with(|e| e.borrow().len()) } else { 0 })));
    // `ENOMOTO_MIP_PROBREP` ([`probrep_on`]): 成果 0 なら、この回に渡した問題の列の状態と含意を控える (成果があれば消す)
    if probrep_on() && !submip && !PRESOLVE_IN_SUBMIP.with(|f| f.get()) {
        let (gf, gt, ge) = PROBE_GAIN.with(|f| f.get());
        // `f` を含めば成果があっても控える (省く道を fuzz で通すための強制。含意は狭めた問題でもそのまま成り立つ)
        let force = env_str!("ENOMOTO_MIP_PROBREP").is_some_and(|v| v.contains('f'));
        let snap = (gf + gt + ge == 0 || force).then(|| ProbRepSnap {
            n: prob.n,
            cost: prob.cost.clone(),
            col_lo: prob.col_lo.clone(),
            col_up: prob.col_up.clone(),
            is_int: prob.is_int.clone(),
            edges: prob.conflict_edges.clone(),
            implied: prob.implied_bounds.clone(),
        });
        PROBREP_LAST.with(|c| *c.borrow_mut() = snap);
    }
    if prbwork {
        let gain = fixed + tightened + PROBE_GAIN.with(|f| f.get().2);
        if gain > 0 {
            PRBWORK_DRY.with(|f| f.set(false));
        } else if cut_dry || env_str!("ENOMOTO_MIP_PRBWORK").is_some_and(|v| v.contains('f')) {
            // 値 `f` (確かめ用、fuzz で省く道を通すため): 打ち切りによらず、成果 0 なら印を付ける
            PRBWORK_DRY.with(|f| f.set(true));
            if verbose {
                eprintln!("MIP: probing: PRBWORK: nothing found within the limit; probing after a restart is skipped in this solve");
            }
        }
    }
    if fixed == 0 && tightened == 0 {
        return None;
    }
    // 締めた境界を問題に入れる (連続列の境界は伝播の誤差の分だけ緩めてある)
    for k in 0..n {
        if dom.lo[k] > prob.col_lo[k] {
            prob.col_lo[k] = dom.lo[k];
        }
        if dom.up[k] < prob.col_up[k] {
            prob.col_up[k] = dom.up[k];
        }
    }
    // 緩めた分で幅が許容誤差以下になった列 (値 0 のはずの連続列が `[0, 1e-9]` など) は 1 点に固定する ([`fix_tiny_width_columns`])
    let tiny = fix_tiny_width_columns(&prob.rows, &mut prob.row_lo, &mut prob.row_up, &mut prob.col_lo, &mut prob.col_up, &prob.is_int, submip, "probe");
    if verbose && tiny > 0 {
        eprintln!("MIP: probing: fixed {tiny} columns of width within tolerance");
    }
    Some(true)
}

/// `ENOMOTO_MIP_PROBE_STRENGTHEN` (試作): プロービングで 0-1 列 `x_j` を `one` の側に固定して伝播した後の境界 `lo`/`up` で、
/// 境界が変わった列 (`ch`) を含む片側の行 (等式・範囲制約の行は除く) の最大 (最小) 活動を求め、右辺との間に隙間 `gap > 0`
/// があれば行の置き換えの候補を `out` に足す (CglProbing の `strengthenRow`、CglProbing.cpp 5893-6060・6677-6772)。
///
/// `Σ a x <= U` で、`x_j = 0` のとき最大活動 `maxR` が `U - gap` なら `Σ a x - gap x_j <= U - gap`、`x_j = 1` のときなら
/// `Σ a x + gap x_j <= U` に置き換える (`>=` の行は向きを逆にして同じ)。`x_j` が 0 と 1 のどちらでも、新しい行は
/// 元の行 (と、`x_j` をその側に固定したとき伝播が示した `Σ a x <= maxR`) と同値なので整数解の集合は変わらず、
/// `x_j ∈ [0, 1]` で新しい行は元の行より強い (LP の緩和が締まる)。伝播の結論は整数解すべてで成り立つので、
/// 別の行の置き換えと組み合わせても正しいが、同じ行を 2 回置き換えると (隙間を元の行で測っているので) 正しくないので、
/// 呼び出し側で行ごとに 1 本だけ残す。候補は `(行, x_j の係数の増分, 下限の増分, 上限の増分)`。
#[allow(clippy::too_many_arguments)]
fn probe_strengthen_collect(prob: &MipProblem, lo: &[f64], up: &[f64], j: usize, one: bool, ch: &[(usize, bool, f64)], mark: &mut [bool], touched: &mut Vec<usize>, out: &mut Vec<(usize, f64, f64, f64)>) {
    // 隙間の下限 (CglProbing は主の許容誤差 1e-7)
    let tol = tunable!("ENOMOTO_T_MIP_PROBE_STRENGTHEN_TOL", 1e-6, f64);
    touched.clear();
    for k in std::iter::once(j).chain(ch.iter().map(|t| t.0)) {
        for &(i, _) in &prob.cols[k] {
            if !mark[i] {
                mark[i] = true;
                touched.push(i);
            }
        }
    }
    for &i in touched.iter() {
        mark[i] = false;
        let (l, u) = (prob.row_lo[i], prob.row_up[i]);
        let le = l == f64::NEG_INFINITY && u.is_finite();
        let ge = u == f64::INFINITY && l.is_finite();
        if !le && !ge {
            continue;
        }
        // 片側の活動 (`<=` の行は最大、`>=` の行は最小)。無限の項があれば候補にしない
        let mut act = 0.0f64;
        let mut finite = true;
        for &(k, a) in &prob.rows[i] {
            let b = if (a > 0.0) == le { up[k] } else { lo[k] };
            if !b.is_finite() {
                finite = false;
                break;
            }
            act += a * b;
        }
        if !finite {
            continue;
        }
        let gap = if le { u - act } else { act - l };
        if !(gap > tol * (1.0 + if le { u.abs() } else { l.abs() }) && gap < 1e8) {
            continue;
        }
        // (係数の増分, 下限の増分, 上限の増分)
        let (dc, dl, du) = match (le, one) {
            (true, false) => (-gap, 0.0, -gap),
            (true, true) => (gap, 0.0, 0.0),
            (false, false) => (gap, gap, 0.0),
            (false, true) => (-gap, 0.0, 0.0),
        };
        out.push((i, dc, dl, du));
    }
}

thread_local! {
    /// 診断 `ENOMOTO_MIP_DEBUG_SUBMIP_COMPONENTS` 用: このスレッドで解いているサブ MIP を出したヒューリスティクスの名前
    pub(crate) static SUBMIP_LABEL: std::cell::Cell<&'static str> = const { std::cell::Cell::new("") };
}

/// 診断 `ENOMOTO_MIP_DEBUG_SUBMIP_COMPONENTS` か。
pub(crate) fn debug_submip_components() -> bool {
    env_str!("ENOMOTO_MIP_DEBUG_SUBMIP_COMPONENTS").is_some()
}

/// 自由な列 (下限 < 上限) だけで見た連結成分の大きさ (列の数、大きい順)。行は自由な列を 2 本以上含むものだけがつなぐ。
/// `drop` が真の列はつながりから外す (分枝で固定したと思った列。成分の数にも数えない)。
fn free_column_components(p: &MipProblem, drop: &[bool]) -> Vec<usize> {
    let n = p.n;
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(par: &mut [usize], mut x: usize) -> usize {
        while par[x] != x {
            par[x] = par[par[x]];
            x = par[x];
        }
        x
    }
    let free = |j: usize| p.col_lo[j] < p.col_up[j] && !drop[j];
    for row in &p.rows {
        let mut first: Option<usize> = None;
        for &(j, _) in row {
            if !free(j) {
                continue;
            }
            match first {
                None => first = Some(j),
                Some(f) => {
                    let (a, b) = (find(&mut parent, f), find(&mut parent, j));
                    if a != b {
                        parent[a] = b;
                    }
                }
            }
        }
    }
    let mut size = vec![0usize; n];
    for j in 0..n {
        if free(j) {
            let r = find(&mut parent, j);
            size[r] += 1;
        }
    }
    let mut s: Vec<usize> = size.into_iter().filter(|&k| k > 0).collect();
    s.sort_unstable_by(|a, b| b.cmp(a));
    s
}

/// 診断 `ENOMOTO_MIP_DEBUG_SUBMIP_COMPONENTS` (既定では使わない): サブ MIP `p` の自由な列の連結成分を 1 行出す。そのまま (`as is`) と、
/// 自由な整数列のうち行の数の多い上位 1%・5% をつながりから外したとき (分枝で先に固定すれば分かれるか)。`stage` は前処理の前か後か
pub(crate) fn report_submip_components(p: &MipProblem, stage: &str) {
    let fmt = |s: &[usize]| {
        let top: Vec<String> = s.iter().take(5).map(|k| k.to_string()).collect();
        let single = s.iter().filter(|&&k| k == 1).count();
        format!("{} components (largest {}, singletons {single}, top [{}])", s.len(), s.first().copied().unwrap_or(0), top.join(","))
    };
    let none = vec![false; p.n];
    let base = free_column_components(p, &none);
    let nfree: usize = base.iter().sum();
    let mut ints: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j] && p.col_lo[j] < p.col_up[j]).collect();
    ints.sort_by_key(|&j| std::cmp::Reverse(p.cols[j].len()));
    let mut parts = vec![format!("as is {}", fmt(&base))];
    for pct in [1usize, 5] {
        let k = (ints.len() * pct).div_ceil(100);
        let mut drop = vec![false; p.n];
        for &j in ints.iter().take(k) {
            drop[j] = true;
        }
        parts.push(format!("top {pct}% ({k} int cols) dropped {}", fmt(&free_column_components(p, &drop))));
    }
    eprintln!("SUBMIP_COMP: {} {stage}: rows {}, free cols {nfree} (int {}); {}", SUBMIP_LABEL.with(|l| l.get()), p.m, ints.len(), parts.join("; "));
}

/// 早めのプロービングとかけ直し (`ENOMOTO_MIP_NO_PROBE_EARLY_REPRESOLVE` で使わない) の短いプロービング: 行の多い 0-1 列から `ENOMOTO_T_MIP_PROBE_EARLY_CANDS` 本 (既定 64)
/// だけ [`probe`] と同じ規則 (片方が矛盾すれば固定、両方の結果の弱い方まで境界を締める) で試し、新たに固定された列が自由な列の
/// `ENOMOTO_T_MIP_PROBE_EARLY_FRAC` (既定 0.1) 以上なら、締めた境界を `prob` に入れてその数を返す (少なければ `prob` は変えず 0)。
/// 含意は集めない。実行不能と分かれば `None`。
/// neos8 では最初の 32 本のプロービング (0.3 秒) で 0-1 列 23035 本のうち 21379 本が固定されるが、既定ではプロービングが前処理の
/// 最後で、かけ直さないので 46070 行・23039 列のまま LP による固定 (4.9 秒) とプロービング (10.5 秒、含意集め) を回し、根も
/// その大きさで解いていた (HiGHS の前処理は 6496 行・659 列。analysis/smallprob_overhead_20261028.md)
fn quick_probe_fixings(prob: &mut MipProblem, verbose: bool, submip: bool) -> Option<usize> {
    let _pt = pretime::guard("qprobe");
    use domain::Domain;
    let t0 = std::time::Instant::now();
    let n = prob.n;
    let free0 = (0..n).filter(|&j| prob.col_lo[j] < prob.col_up[j]).count();
    let mut dom = Domain::new(prob);
    if !dom.propagate(prob) {
        return None;
    }
    let mut cands: Vec<usize> = (0..n).filter(|&j| prob.is_int[j] && dom.lo[j] == 0.0 && dom.up[j] == 1.0).collect();
    cands.sort_by_key(|&j| std::cmp::Reverse(prob.cols[j].len()));
    let max_cands = tunable!("ENOMOTO_T_MIP_PROBE_EARLY_CANDS", 64usize, usize);
    let mut probed = 0usize;
    for &j in &cands {
        if probed >= max_cands {
            break;
        }
        if dom.lo[j] == dom.up[j] {
            continue;
        }
        probed += 1;
        let pos = dom.stack_len();
        dom.tighten_upper(prob, j, 0.0);
        let ok0 = dom.propagate(prob);
        let ch0: Vec<_> = if ok0 { dom.changes_since(pos).collect() } else { Vec::new() };
        dom.backtrack_to(prob, pos);
        dom.tighten_lower(prob, j, 1.0);
        let ok1 = dom.propagate(prob);
        let ch1: Vec<_> = if ok1 { dom.changes_since(pos).collect() } else { Vec::new() };
        dom.backtrack_to(prob, pos);
        match (ok0, ok1) {
            (false, false) => return None,
            (false, true) | (true, false) => {
                let v = if ok1 { 1.0 } else { 0.0 };
                dom.tighten_lower(prob, j, v);
                dom.tighten_upper(prob, j, v);
                if !dom.propagate(prob) {
                    return None;
                }
            }
            (true, true) => {
                let mut first: std::collections::HashMap<(usize, bool), f64> = std::collections::HashMap::new();
                for &(k, upper, v) in &ch0 {
                    first.insert((k, upper), v);
                }
                let mut last1: std::collections::HashMap<(usize, bool), f64> = std::collections::HashMap::new();
                for &(k, upper, v) in &ch1 {
                    last1.insert((k, upper), v);
                }
                let mut any = false;
                for ((k, upper), v0) in hm_entries(&first) {
                    if k == j {
                        continue;
                    }
                    let Some(&v1) = last1.get(&(k, upper)) else { continue };
                    if upper {
                        let u = v0.max(v1);
                        if u < dom.up[k] - 1e-9 * (1.0 + u.abs()) {
                            dom.tighten_upper(prob, k, u);
                            any = true;
                        }
                    } else {
                        let l = v0.min(v1);
                        if l > dom.lo[k] + 1e-9 * (1.0 + l.abs()) {
                            dom.tighten_lower(prob, k, l);
                            any = true;
                        }
                    }
                }
                if any && !dom.propagate(prob) {
                    return None;
                }
            }
        }
    }
    let newly = (0..n).filter(|&j| prob.col_lo[j] < prob.col_up[j] && dom.lo[j] == dom.up[j]).count();
    let use_it = newly > 0 && newly as f64 >= tunable!("ENOMOTO_T_MIP_PROBE_EARLY_FRAC", 0.1, f64) * free0 as f64;
    if verbose {
        eprintln!("MIP: early probing: {probed} probed, {newly} of {free0} free columns fixed ({:.2}s){}", t0.elapsed().as_secs_f64(), if use_it { ", presolve again" } else { ", not used" });
    }
    if !use_it {
        return Some(0);
    }
    // 整数列は丸めた境界、連続列は伝播の誤差の分だけ緩めた境界 ([`probe`] と同じ)
    for k in 0..n {
        if dom.lo[k] > prob.col_lo[k] {
            prob.col_lo[k] = dom.lo[k];
        }
        if dom.up[k] < prob.col_up[k] {
            prob.col_up[k] = dom.up[k];
        }
    }
    fix_tiny_width_columns(&prob.rows, &mut prob.row_lo, &mut prob.row_up, &mut prob.col_lo, &mut prob.col_up, &prob.is_int, submip, "quick");
    Some(newly)
}

/// HiGHS 1.15.1 の Enumeration 前処理 (`HPresolve::enumerateSolutions`) と同じ規則の行の列挙 (`ENOMOTO_MIP_ENUM_PRESOLVE`、試作)。
/// 固定されていない列がすべて 0-1 で 2〜8 本の行を選び (列の行の数の平均の大きい順、列の重なりが 5 割を超える行は後ろの方を外し、
/// 最大 400 行)、行の列のすべての割り当てを伝播つきで列挙する。どの実行可能な割り当てでも締まる境界 (各解の境界の弱い方) を返し、
/// 行の中でどの解でも同じ値・逆の値の列の組を `equiv` に足す。何も見つからない行が 6 行続いたらやめる (HiGHS と同じ)。
/// 実行可能な割り当てのない行があれば `None` (問題は実行不能)。返り値は (締める境界、列挙した行の数、解の数の合計)。
/// physiciansched6-2 では HiGHS の前処理のこの規則を切ると、HiGHS の前処理後の問題に対する ENOMOTO の根の LP が 48974 -> 48374 に下がる
/// 試作 `ENOMOTO_MIP_ENUM_WHERE` (`ENOMOTO_MIP_ENUM_PRESOLVE` と組み合わせる。analysis/cutgap_20261123 の 7 節): 列挙の前処理を置く場所。
/// `before` = プロービングの前だけ、`after` (既定) = プロービングの後だけ、`loop` = プロービングの後に回し、その回 (プロービングと列挙) で新しい固定・締め付け・
/// 同値な組・衝突の組 (clique) が出れば、プロービングと列挙をもう一度回す、を何も出なくなるまで繰り返す ([`enum_probe_loop`]。回数の上限は置かない。
/// 止めは前処理の時間の止め [`presolve_stop`] だけ)。`loop` では列挙で分かった衝突の組を含意の表に足す (足さないと次の回も同じ組を新しいと数える)
/// 試作 `ENOMOTO_MIP_ENUM_ADAPT` (既定では使わない。analysis/cutgap_20261123 の 8 節): 列挙の前処理 (`ENOMOTO_MIP_ENUM_PRESOLVE=all`)・
/// 列挙とプロービングの繰り返し (`ENOMOTO_MIP_ENUM_WHERE=loop`)・締め付けでの前処理のかけ直し (`ENOMOTO_MIP_PRR_TIGHT`) を全部使い、
/// 2 周目からの繰り返しとかけ直しを続けるか、次の列挙で何割の行を列挙するかを、直前の周の成果と時間からシグモイドで決める ([`enum_adapt_continue`])
fn enum_adapt_on() -> bool {
    env_str!("ENOMOTO_MIP_ENUM_ADAPT").is_some_and(|v| v != "0")
}

/// 列挙の前処理の形 (`ENOMOTO_MIP_ENUM_PRESOLVE` の値。[`enum_adapt_on`] なら `all`)。`None` は使わない
fn enum_presolve_mode() -> Option<&'static str> {
    if enum_adapt_on() {
        return Some("all");
    }
    env_str!("ENOMOTO_MIP_ENUM_PRESOLVE")
}

/// `ENOMOTO_MIP_ENUM_ADAPT=log`: 判定を出すだけで常に続ける (繰り返しは以前の規則で止まる。係数を決めるためのデータ集め)
fn enum_adapt_log() -> bool {
    env_str!("ENOMOTO_MIP_ENUM_ADAPT") == Some("log")
}

/// 求解 (再スタートの前の最初の求解) の始めからの秒 ([`ENUM_T0`]。置かれていなければ無限大)
fn enum_elapsed() -> f64 {
    ENUM_T0.with(|c| c.get()).map_or(f64::INFINITY, |t| t.elapsed().as_secs_f64())
}

/// 求解の始め ([`solve_problem_inner`]、サブ MIP でない最初の求解) に、[`enum_adapt_on`] の時計と列挙の記録を置き直す
pub(super) fn enum_adapt_reset(t0: std::time::Instant) {
    ENUM_T0.with(|c| c.set(Some(t0)));
    ENUM_STEP_LAST.with(|c| c.set(None));
}

/// [`enum_adapt_on`] の判定: 直前の周 (列挙 1 回、列挙とプロービングの繰り返しの 1 回、またはかけ直しの前の繰り返し全体) が、列 `n` 本の問題で
/// `gain` 個を `secs` 秒で得た (周の始めの求解の経過時間 `el` 秒) とき、得の割合 g = gain / n と時間の割合 c = secs / el から
/// s = log10(g / c) (それまでの求解の時間の割合あたりの得)、f = sigmoid(a (s − b)) とし、次の周が直前の周と同じだけ時間を使うとして、
/// c が f 以下なら続ける (f は、それまでの求解の時間に対して次の周に使ってよい割合)。続けるなら、次の列挙で行の f の割合を列挙する ([`ENUM_ADAPT_ROWS`])。
/// 得は試作が新しく得たものだけ: 列挙の固定・締め付け・同値な組と、繰り返しの 2 周目からのプロービングの固定・締め付け・新しい同値な組
/// (1 周目のプロービングは既定でも回る。衝突の組は数えない)。a = `ENOMOTO_T_MIP_ENUM_ADAPT_A`、b = `ENOMOTO_T_MIP_ENUM_ADAPT_B` は
/// analysis/cutgap_20261123 の 9 節の 9 問の判定の (s, c) から決めた。時計が無ければ c = 0 で常に続ける
fn enum_adapt_continue(gain: usize, n: usize, secs: f64, el: f64, verbose: bool, what: &str) -> bool {
    if gain == 0 {
        if verbose {
            eprintln!("MIP: ENUM_ADAPT: {what}: gain 0 in {secs:.2}s ({el:.2}s elapsed) -> stop");
        }
        ENUM_ADAPT_ROWS.with(|r| r.set(None));
        return enum_adapt_log();
    }
    let g = gain as f64 / n.max(1) as f64;
    let c = if el.is_finite() && el > 0.0 { secs / el } else { 0.0 };
    let s = if c > 0.0 { (g / c).log10() } else { f64::INFINITY };
    let (a, b) = (tunable!("ENOMOTO_T_MIP_ENUM_ADAPT_A", 8.0, f64), tunable!("ENOMOTO_T_MIP_ENUM_ADAPT_B", -0.09, f64));
    let f = if s.is_finite() { 1.0 / (1.0 + (-(a * (s - b)).clamp(-50.0, 50.0)).exp()) } else { 1.0 };
    let go = c <= f;
    ENUM_ADAPT_ROWS.with(|r| r.set((go && !enum_adapt_log()).then_some(f)));
    if verbose {
        eprintln!("MIP: ENUM_ADAPT: {what}: gain {gain} of {n} columns in {secs:.2}s ({el:.2}s elapsed): g {g:.4} c {c:.4} s {s:.3} f {f:.4} -> {}", if go { "continue" } else { "stop" });
    }
    go || enum_adapt_log()
}

/// 試作 `ENOMOTO_MIP_ENUM_ADAPT` の `s` (値に `s` を含む、analysis/comboharm_20261124): 列挙 1 回 ([`enumerate_rows`]) の途中でも、
/// [`enum_adapt_continue`] と同じシグモイドで止めるかを決める。始めてから `secs` 秒で `gain` 個 (固定・締め付け・同値な組) を得たとき、
/// c = secs / el (el は列挙の始めの求解の経過時間)、f = sigmoid(a (log10(g / c) − b)) とし、c > max(f, 1/2) なら止める。
/// 1 周目には前の周の観測がないので、得がまだない間 (と得が少ない間) は中立の点 (s = b、f = 1/2) を下限にする: 得がないまま
/// 求解の経過時間の半分を列挙に使ったら止める。時計が無ければ止めない
fn enum_adapt_midstop(gain: usize, n: usize, secs: f64, el: f64) -> bool {
    if !(el.is_finite() && el > 0.0) {
        return false;
    }
    let c = secs / el;
    let f = if gain == 0 {
        0.5
    } else {
        let s = ((gain as f64 / n.max(1) as f64) / c.max(1e-300)).log10();
        let (a, b) = (tunable!("ENOMOTO_T_MIP_ENUM_ADAPT_A", 8.0, f64), tunable!("ENOMOTO_T_MIP_ENUM_ADAPT_B", -0.09, f64));
        1.0 / (1.0 + (-(a * (s - b)).clamp(-50.0, 50.0)).exp())
    };
    c > f.max(0.5)
}

fn enum_adapt_mid_on() -> bool {
    enum_adapt_on() && env_str!("ENOMOTO_MIP_ENUM_ADAPT").is_some_and(|v| v.contains('s'))
}

fn enum_where() -> &'static str {
    if enum_adapt_on() {
        return "loop";
    }
    match env_str!("ENOMOTO_MIP_ENUM_WHERE") {
        Some("before") => "before",
        Some("loop") => "loop",
        _ => "after",
    }
}

thread_local! {
    /// `ENOMOTO_MIP_ENUM_WHERE=loop`: [`enum_probe_loop`] の中の [`probe`] か (入れ子で回さない)
    static ENUM_LOOP_IN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 最後の [`enum_presolve_step`] の成果 (固定, 締め付け, 新しい同値な組, 新しい衝突の組)
    static ENUM_GAIN: std::cell::Cell<(usize, usize, usize, usize)> = const { std::cell::Cell::new((0, 0, 0, 0)) };
    /// [`enum_probe_loop`] の中で、列挙がこれまでの回に見つけた衝突の組 (回をまたいで「新しい」を数えるため。ループの外では `None`)。
    /// 含意の表は回の間のプロービング (CLQPRB=d など) で作り直されて列挙の組が落ちることがあり、表と比べるだけでは同じ組を毎回「新しい」と数えて
    /// ループが止まらなかった (mzzv42z: 3 回目からは毎回 728 組を「新しい」と数え、前処理の時間の止めまで 17 回、60 秒。8 節)
    static ENUM_EDGES_SEEN: std::cell::RefCell<Option<std::collections::HashSet<(u32, u32)>>> = const { std::cell::RefCell::new(None) };
    /// [`enum_adapt_on`]: 次の列挙で列挙する行の割合 (`None` は全部)。[`enum_adapt_continue`] が置き、[`enum_probe_loop`] の始めと終わりに消す
    static ENUM_ADAPT_ROWS: std::cell::Cell<Option<f64>> = const { std::cell::Cell::new(None) };
    /// 最後の [`enum_probe_loop`] の (得 = 固定・締め付け・新しい同値な組の和, 秒, 始めの残り時間の秒)。[`represolve_after_probe`] の判定に使う
    static ENUM_LOOP_LAST: std::cell::Cell<(usize, f64, f64)> = const { std::cell::Cell::new((0, 0.0, f64::INFINITY)) };
    /// [`enum_adapt_on`] の時計: 求解の始め ([`enum_adapt_reset`])
    static ENUM_T0: std::cell::Cell<Option<std::time::Instant>> = const { std::cell::Cell::new(None) };
    /// [`enum_adapt_on`]: この求解で最後に回した列挙の (得 = 固定・締め付け・同値な組, 秒, 始めの経過時間の秒)。次の列挙を回すかの判定に使う
    /// (再スタートの後の前処理にも引き継ぐ)
    static ENUM_STEP_LAST: std::cell::Cell<Option<(usize, f64, f64)>> = const { std::cell::Cell::new(None) };
}

/// `ENOMOTO_MIP_ENUM_WHERE=loop` ([`enum_where`]): [`probe`] (中でプロービングと列挙) を、新しい固定・締め付け・同値な組・衝突の組が出なくなるまで繰り返す。
/// 固定・締め付けは各回で問題の境界に入り、衝突の組は問題の含意として次の回に引き継がれる。同値な組は回をまたいで重複を除いて集める
fn enum_probe_loop(prob: &mut MipProblem, verbose: bool, submip: bool) -> Option<bool> {
    let t0 = std::time::Instant::now();
    let mut merged: Vec<(usize, usize, bool)> = Vec::new();
    let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    let (mut force, mut any, mut tot_fixed, mut tot_tight, mut it) = (false, false, 0usize, 0usize, 0usize);
    ENUM_EDGES_SEEN.with(|c| *c.borrow_mut() = Some(std::collections::HashSet::new()));
    ENUM_ADAPT_ROWS.with(|r| r.set(None));
    let el0 = enum_elapsed();
    let mut tot_gain = 0usize;
    let r = loop {
        it += 1;
        ENUM_GAIN.with(|g| g.set((0, 0, 0, 0)));
        let (tr, el) = (std::time::Instant::now(), enum_elapsed());
        let r = probe(prob, verbose, submip);
        if r == Some(false) {
            break Some(false);
        }
        any |= r == Some(true);
        force |= PROBE_EQUIV_FORCE.with(|f| f.get());
        let eq = PROBE_EQUIV.with(|e| std::mem::take(&mut *e.borrow_mut()));
        let mut new_eq = 0usize;
        for (j, k, neg) in eq {
            if seen.insert((j.min(k), j.max(k))) {
                merged.push((j, k, neg));
                new_eq += 1;
            }
        }
        let (gf, gt, _) = PROBE_GAIN.with(|f| f.get());
        let (ef, et, ee, ge) = ENUM_GAIN.with(|g| g.get());
        tot_fixed += PROBE_FIXED.with(|f| f.get());
        tot_tight += gt;
        if verbose {
            eprintln!("MIP: ENUM_WHERE=loop: round {it}: {gf} fixed, {gt} tightened, {new_eq} new equivalent pairs, {ge} new conflict pairs ({:.2}s)", t0.elapsed().as_secs_f64());
        }
        // 次の周の材料になる成果: 固定・締め付け (列挙とプロービング。定義域が変わると次のプロービング・列挙が新しく見つけうる) と列挙の同値な組。
        // プロービングの同値な組は、1 周目は既定でも見つかるので数えず、2 周目からは数える
        let gain = gf + gt + ef + et + ee + if it == 1 { 0 } else { new_eq };
        tot_gain += gain;
        if gf + gt + new_eq + ge == 0 || presolve_stop() {
            break if any { Some(true) } else { None };
        }
        // `ENOMOTO_MIP_ENUM_ADAPT`: 次の周を回すかをこの周の成果と時間で決める
        if enum_adapt_on() && !enum_adapt_continue(gain, prob.n, tr.elapsed().as_secs_f64(), el, verbose, &format!("loop round {it}")) {
            break if any { Some(true) } else { None };
        }
    };
    ENUM_LOOP_LAST.with(|c| c.set((tot_gain, t0.elapsed().as_secs_f64(), el0)));
    ENUM_ADAPT_ROWS.with(|r| r.set(None));
    ENUM_EDGES_SEEN.with(|c| *c.borrow_mut() = None);
    PROBE_EQUIV.with(|e| *e.borrow_mut() = merged);
    PROBE_EQUIV_FORCE.with(|f| f.set(force));
    PROBE_FIXED.with(|f| f.set(tot_fixed));
    let ne = PROBE_EQUIV.with(|e| e.borrow().len());
    PROBE_GAIN.with(|f| f.set((tot_fixed, tot_tight, ne)));
    r
}

/// 列挙の前処理 ([`enumerate_rows`]) を 1 回回し、見つけた境界を `dom` に入れる。`ENOMOTO_MIP_ENUM_PRESOLVE` の `impl`・`allimpl` は含意 `edges` を伝播に使う
/// 写しで列挙する。衝突の組は数え、`ENOMOTO_MIP_ENUM_EDGES` か `ENOMOTO_MIP_ENUM_WHERE=loop` なら `edges` に足す。返り値は (固定, 締め付け) (実行不能なら `None`)
#[allow(clippy::too_many_arguments)]
fn enum_presolve_step(prob: &MipProblem, dom: &mut domain::Domain, bin0: &[bool], want_equiv: bool, equiv: &mut Vec<(usize, usize, bool)>, edges: &mut Vec<(u32, u32)>, verbose: bool, where_: &str) -> Option<(usize, usize)> {
    let mode = enum_presolve_mode().unwrap_or("1");
    let n = prob.n;
    // `ENOMOTO_MIP_ENUM_ADAPT`: この求解の前の列挙の得と時間から、今回の列挙を回すかを決める (この求解の最初の列挙は回す)
    if enum_adapt_on() {
        if let Some((g, secs, el)) = ENUM_STEP_LAST.with(|c| c.get()) {
            if !enum_adapt_continue(g, n, secs, el, verbose, &format!("enumeration ({where_})")) {
                return Some((0, 0));
            }
        }
    }
    let te = std::time::Instant::now();
    let el_step = enum_elapsed();
    let e0 = equiv.len();
    let (mut fixed, mut tightened) = (0usize, 0usize);
    let mut d2 = if (mode == "impl" || mode == "allimpl") && !edges.is_empty() {
        let mut d = dom.clone();
        d.add_implications(n, edges);
        Some(d)
    } else {
        None
    };
    // `ENOMOTO_MIP_ENUM_EDGES`: 列挙で分かった衝突の組を含意の表 (クリークのグラフ、クリーク行の拡張が使う) に足す
    let want_edges = env_str!("ENOMOTO_MIP_ENUM_EDGES").is_some() || enum_where() == "loop";
    let mut new_edges: Vec<(u32, u32)> = Vec::new();
    // 列挙の前の境界 ([`enumerate_rows`] は見つけた締め付けをその場で `dom` に入れるので、下で入れ直すときに「締まった」と数えるため)
    let (pre_lo, pre_up) = (dom.lo.clone(), dom.up.clone());
    let res = enumerate_rows(prob, d2.as_mut().unwrap_or(dom), bin0, if want_equiv { Some(&mut *equiv) } else { None }, Some(&mut new_edges));
    let have: std::collections::HashSet<(u32, u32)> = edges.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect();
    new_edges.sort_unstable();
    new_edges.dedup();
    new_edges.retain(|e| !have.contains(e));
    // ループの中では、前の回に見つけた組も「新しい」と数えない (含意の表に足すのは表に無い組全部のまま)
    let n_edges = ENUM_EDGES_SEEN.with(|c| match c.borrow_mut().as_mut() {
        Some(seen) => new_edges.iter().filter(|&&e| seen.insert(e)).count(),
        None => new_edges.len(),
    });
    if want_edges {
        edges.extend(new_edges);
    }
    let (wc, rows_done, sols) = res?;
    // 見つけた境界を元の定義域に入れる (`impl` の写しで見つけたものも大域的に成り立つ)
    for (k, upper, v) in wc {
        let was_fixed = pre_lo[k] == pre_up[k];
        let ch = if upper { dom.tighten_upper(prob, k, v) } else { dom.tighten_lower(prob, k, v) };
        // 列挙の中で入れ済みの締め付け (`ch` が偽) も、列挙の前より締まっていれば数える (以前は数えず、neos-960392 で 84 本の締め付けを
        // 「0 固定・0 締め付け」と出し、`HO3_ENUM` の `a` が「何も見つからなかった」と誤って次の回を飛ばしていた。treeclose 9.57 節)
        let ch = ch || if upper { dom.up[k] < pre_up[k] } else { dom.lo[k] > pre_lo[k] };
        if ch {
            if !was_fixed && dom.lo[k] == dom.up[k] {
                fixed += 1;
            } else {
                tightened += 1;
            }
        }
    }
    if !dom.propagate(prob) {
        return None;
    }
    let ne = equiv.len() - e0;
    if enum_adapt_on() {
        ENUM_STEP_LAST.with(|c| c.set(Some((fixed + tightened + ne, te.elapsed().as_secs_f64(), el_step))));
    }
    ENUM_GAIN.with(|g| {
        let (a, b, c, d) = g.get();
        g.set((a + fixed, b + tightened, c + ne, d + if want_edges { n_edges } else { 0 }));
    });
    if verbose {
        eprintln!(
            "MIP: enumeration presolve ({where_}): {rows_done} rows, {sols} solutions, {fixed} fixed, {tightened} bounds tightened, {ne} equivalent pairs, {n_edges} new conflict pairs{} ({:.2}s)",
            if want_edges { "" } else { " (not added)" },
            te.elapsed().as_secs_f64()
        );
    }
    Some((fixed, tightened))
}

fn enumerate_rows(prob: &MipProblem, dom: &mut domain::Domain, bin0: &[bool], mut equiv: Option<&mut Vec<(usize, usize, bool)>>, mut edges: Option<&mut Vec<(u32, u32)>>) -> Option<(Vec<(usize, bool, f64)>, usize, usize)> {
    const MAX_ROW: usize = 8;
    // `ENOMOTO_MIP_ENUM_PRESOLVE=all` (試作、analysis/cutgap_20261123 の 2 節): 行の数の上限と空振りでやめる規則を外し、候補の行を全部列挙する
    // (止めるのは前処理の時間の止め [`presolve_stop`] だけ)。net12 は上限つきでは 30 行で空振り 6 回に当たって何も見つけず、全部 (953 行・0.05 秒) で
    // 根の LP が 68.4 → 128.3 (HiGHS の Enumeration 前処理と同じ)
    let all = enum_presolve_mode().is_some_and(|v| v.starts_with("all"));
    let max_rows = if all { usize::MAX } else { tunable!("ENOMOTO_T_MIP_ENUM_ROWS", 400usize, usize) };
    let max_fails = if all { usize::MAX } else { tunable!("ENOMOTO_T_MIP_ENUM_FAILS", 6usize, usize) };
    // 試作 `ENOMOTO_MIP_ENUM_ADAPT` の `s` ([`enum_adapt_midstop`]): 列挙の途中の得 (固定・締め付け・同値な組。衝突の組は数えない) と時間。
    // 時計は候補の行を並べ、重なりの多い行を外す所 (行の数の 2 乗の手間。neos-950242 は 28968 行で約 4 秒) から数える
    let mid = enum_adapt_mid_on();
    let (te0, el0, mut gain_mid) = (std::time::Instant::now(), enum_elapsed(), 0usize);
    let row_vars = |dom: &domain::Domain, i: usize| -> Option<Vec<usize>> {
        let mut v = Vec::new();
        for &(j, _) in &prob.rows[i] {
            if dom.lo[j] == dom.up[j] {
                continue;
            }
            if !bin0[j] || v.len() >= MAX_ROW {
                return None;
            }
            v.push(j);
        }
        if v.len() < 2 {
            return None;
        }
        v.sort_unstable();
        Some(v)
    };
    // 候補の行 (HiGHS は列のプロービングの点数の平均で並べる。ここでは列の行の数の平均)
    let mut rows: Vec<(f64, usize, Vec<usize>)> = (0..prob.m)
        .filter_map(|i| row_vars(dom, i).map(|v| (v.iter().map(|&j| prob.cols[j].len() as f64).sum::<f64>() / v.len() as f64, i, v)))
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    // `ENOMOTO_MIP_ENUM_ADAPT`: 繰り返しの 2 周目からは、前の周で決めた割合 f の行 (並べた順の先頭から) だけを列挙する
    let max_rows = match ENUM_ADAPT_ROWS.with(|r| r.get()) {
        Some(f) => max_rows.min(((f * rows.len() as f64).ceil() as usize).max(1)),
        None => max_rows,
    };
    // 重なりの多い行を外す
    let mut keep: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut removed = vec![false; rows.len()];
    for a in 0..rows.len() {
        if removed[a] {
            continue;
        }
        keep.push((rows[a].1, rows[a].2.clone()));
        if keep.len() >= max_rows {
            break;
        }
        // `ENOMOTO_MIP_ENUM_ADAPT` の `s`: 重なりを外す間も (得はまだ 0 として) 同じ判定を当てる
        if mid && keep.len() % 16 == 0 && enum_adapt_midstop(0, prob.n, te0.elapsed().as_secs_f64(), el0) {
            if env_str!("ENOMOTO_MIP_LOG").is_some() {
                eprintln!("MIP: ENUM_ADAPT s: stopped choosing the rows after {} of {} candidate rows ({:.2}s, {el0:.2}s elapsed at the start)", keep.len(), rows.len(), te0.elapsed().as_secs_f64());
            }
            break;
        }
        let va = &rows[a].2;
        for b in a + 1..rows.len() {
            if removed[b] {
                continue;
            }
            let vb = &rows[b].2;
            let (mut ia, mut ib, mut ov) = (0, 0, 0);
            while ia < va.len() && ib < vb.len() {
                match va[ia].cmp(&vb[ib]) {
                    std::cmp::Ordering::Less => ia += 1,
                    std::cmp::Ordering::Greater => ib += 1,
                    std::cmp::Ordering::Equal => {
                        ia += 1;
                        ib += 1;
                        ov += 1;
                    }
                }
            }
            if 100 * ov / va.len().min(vb.len()) > 50 {
                removed[b] = true;
            }
        }
    }
    // 列挙 (深さ優先、割り当てるたびに伝播)
    fn rec(prob: &MipProblem, dom: &mut domain::Domain, vars: &[usize], idx: usize, pos0: usize, sols: &mut Vec<u32>, wc: &mut Option<std::collections::HashMap<(usize, bool), f64>>) {
        let mut i = idx;
        while i < vars.len() && dom.lo[vars[i]] == dom.up[vars[i]] {
            i += 1;
        }
        if i == vars.len() {
            let mut mask = 0u32;
            for (t, &j) in vars.iter().enumerate() {
                if dom.lo[j] >= 0.5 {
                    mask |= 1 << t;
                }
            }
            sols.push(mask);
            // この解で締まった境界 (同じ列の側が何度も変わったら一番締まった値)
            let mut cur: std::collections::HashMap<(usize, bool), f64> = std::collections::HashMap::new();
            for (k, upper, v) in dom.changes_since(pos0) {
                let e = cur.entry((k, upper)).or_insert(v);
                *e = if upper { e.min(v) } else { e.max(v) };
            }
            match wc {
                None => *wc = Some(cur),
                Some(w) => {
                    w.retain(|key, v| match cur.get(key) {
                        Some(&c) => {
                            *v = if key.1 { v.max(c) } else { v.min(c) };
                            true
                        }
                        None => false,
                    });
                }
            }
            return;
        }
        let j = vars[i];
        for val in [0.0, 1.0] {
            let pos = dom.stack_len();
            if val == 0.0 {
                dom.tighten_upper(prob, j, 0.0);
            } else {
                dom.tighten_lower(prob, j, 1.0);
            }
            if dom.propagate(prob) {
                rec(prob, dom, vars, i + 1, pos0, sols, wc);
            }
            dom.backtrack_to(prob, pos);
        }
    }
    let mut out: Vec<(usize, bool, f64)> = Vec::new();
    let (mut fails, mut done, mut nsol) = (0usize, 0usize, 0usize);
    for (i, _) in &keep {
        if done % 16 == 0 && presolve_stop() {
            break;
        }
        if mid && done > 0 && done % 16 == 0 && enum_adapt_midstop(gain_mid, prob.n, te0.elapsed().as_secs_f64(), el0) {
            if env_str!("ENOMOTO_MIP_LOG").is_some() {
                eprintln!("MIP: ENUM_ADAPT s: stopped the enumeration after {done} of {} rows: gain {gain_mid} in {:.2}s ({el0:.2}s elapsed at the start)", keep.len(), te0.elapsed().as_secs_f64());
            }
            break;
        }
        let Some(vars) = row_vars(dom, *i) else { continue };
        done += 1;
        let pos0 = dom.stack_len();
        let mut sols: Vec<u32> = Vec::new();
        let mut wc: Option<std::collections::HashMap<(usize, bool), f64>> = None;
        rec(prob, dom, &vars, 0, pos0, &mut sols, &mut wc);
        if sols.is_empty() {
            return None;
        }
        nsol += sols.len();
        let mut found = 0usize;
        // どの解でも締まる境界は、この先の行の列挙にも使う
        let mut tight: Vec<(usize, bool, f64)> = Vec::new();
        for ((k, upper), v) in wc.unwrap_or_default() {
            if (upper && v < dom.up[k] - 1e-9 * (1.0 + v.abs())) || (!upper && v > dom.lo[k] + 1e-9 * (1.0 + v.abs())) {
                tight.push((k, upper, v));
            }
        }
        for &(k, upper, v) in &tight {
            if upper {
                dom.tighten_upper(prob, k, v);
            } else {
                dom.tighten_lower(prob, k, v);
            }
        }
        if !tight.is_empty() && !dom.propagate(prob) {
            return None;
        }
        found += tight.len();
        gain_mid += tight.len();
        out.extend(tight);
        if let Some(eq) = equiv.as_deref_mut() {
            for a in 0..vars.len() {
                for b in a + 1..vars.len() {
                    if dom.lo[vars[a]] == dom.up[vars[a]] || dom.lo[vars[b]] == dom.up[vars[b]] {
                        continue;
                    }
                    let same = sols.iter().all(|&s| ((s >> a) & 1) == ((s >> b) & 1));
                    let comp = sols.iter().all(|&s| ((s >> a) & 1) != ((s >> b) & 1));
                    if same || comp {
                        eq.push((vars[a], vars[b], comp));
                        found += 1;
                        gain_mid += 1;
                    }
                }
            }
        }
        // どの解にも現れない文字の組 (x_a = va かつ x_b = vb) は衝突の組 (HiGHS は列挙で行がクリークになると分かればクリーク表に入れる)。
        // 文字は 2j (x_j = 1)・2j + 1 (x_j = 0)
        if let Some(ed) = edges.as_deref_mut() {
            for a in 0..vars.len() {
                for b in a + 1..vars.len() {
                    if dom.lo[vars[a]] == dom.up[vars[a]] || dom.lo[vars[b]] == dom.up[vars[b]] {
                        continue;
                    }
                    for va in 0..2u32 {
                        for vb in 0..2u32 {
                            if !sols.iter().any(|&s| ((s >> a) & 1) == va && ((s >> b) & 1) == vb) {
                                let la = 2 * vars[a] as u32 + (1 - va);
                                let lb = 2 * vars[b] as u32 + (1 - vb);
                                ed.push((la.min(lb), la.max(lb)));
                                found += 1;
                            }
                        }
                    }
                }
            }
        }
        if found == 0 {
            fails += 1;
            if fails > max_fails {
                break;
            }
        } else {
            fails = 0;
        }
    }
    Some((out, done, nsol))
}

/// 幅の極めて小さな列の固定 (既定で使う。`ENOMOTO_MIP_NO_TINY_WIDTH_FIX` で使わない): 幅 `up - lo` が許容誤差
/// (`ENOMOTO_T_MIP_TINY_WIDTH_TOL`、既定は MIP の実行可能性の許容誤差 [`domain::FEASTOL`] = 1e-6) 以下の列を 1 点に固定する
/// (許容誤差の内の交差 `lo > up` も同じ)。HiGHS 1.15.1 の前処理 (`HPresolve::checkColBounds`) は幅が
/// `mip_feasibility_tolerance` 以下で、幅が `small_matrix_value` (1e-9) 以下か列の最大の |係数| × 幅が許容誤差以下の列を
/// 下限で固定する。プロービング・伝播は連続列の境界を伝播の誤差の分 (1e-9) だけ緩めて入れるので、値 0 のはずの列が
/// `[0, 1e-9]` で残り、許容誤差の範囲で冗長な行 (`x + y <= 1 - 1e-9`) を係数の強化が縮めて最適解を厳密に切っていた
/// (uccase12 の再スタート後、analysis/equiv2_bug_20261022.md)。固定してしまえば幅の小さな列はどの手順からも見えない。
///
/// - 固定する点: 区間に整数 (0 を含む) があればそれ (伝播が緩める前の値)、なければ下限 (HiGHS と同じ)。目的の係数の側の端は
///   使わない: 緩めた側の端 (元の問題では値を取れない) に固定すると、連続列の等式の連鎖が 1e-8 の大きさで破れ、前処理
///   (許容誤差 1e-9) が実行不能と判定する (benchmarks/tinywidth_20261022 の fuzz_mixtiny、seed 68 など)。
///   整数列は丸め済みなので、交差か幅の小さな区間が 1 つの整数を囲むときだけ当たる。
/// - 行の扱い: 固定で列の値は固定する点から最大 `s_j` (遠い方の端までの距離) 動く。不等式の行は失う余裕 `|a_ij| s_j` だけ
///   右辺を緩める (元の問題で厳密に実行可能な点は 1 つも切らない。緩めないと `x + y <= 1 - 2e-9` が整数だけの行
///   `x <= 1 - 2e-9` になり、整数の行の丸め (許容誤差 1e-9) が x = 1 を切る)。各行の緩めの和 `Σ |a_ij| s_j` は許容誤差まで
///   (越える列は固定しない。緩めには丸めの余裕 1e-9 (1 + |b|) を足す)。等式の行は緩められないので、破れの和が `1e-9 (1 + |b|)` (前処理の許容誤差) までの列だけ固定する。
///
/// 固定した列の数を返す。
pub(crate) fn fix_tiny_width_columns(rows: &[Vec<(usize, f64)>], row_lo: &mut [f64], row_up: &mut [f64], lo: &mut [f64], up: &mut [f64], is_int: &[bool], submip: bool, site: &'static str) -> usize {
    if env_str!("ENOMOTO_MIP_NO_TINY_WIDTH_FIX").is_some() {
        return 0;
    }
    let tol = tunable!("ENOMOTO_T_MIP_TINY_WIDTH_TOL", domain::FEASTOL, f64);
    let n = lo.len();
    // 固定する候補 (列, 点, 遠い方の端までの距離)
    let mut cand: Vec<(usize, f64, f64)> = Vec::new();
    for j in 0..n {
        let (l, u) = (lo[j], up[j]);
        if !(l.is_finite() && u.is_finite()) || l == u || u - l > tol || l - u > tol {
            continue;
        }
        let (a, b) = (l.min(u), l.max(u));
        let k = (0.5 * (a + b)).round();
        let v = if is_int[j] || (a <= k && k <= b) {
            k
        } else if l > u {
            0.5 * (l + u)
        } else {
            l
        };
        let s = (v - a).abs().max((b - v).abs());
        if s > tol {
            continue;
        }
        cand.push((j, v, s));
    }
    if cand.is_empty() {
        return 0;
    }
    let mut is_cand = vec![usize::MAX; n];
    for (c, &(j, _, _)) in cand.iter().enumerate() {
        is_cand[j] = c;
    }
    let mut cols: Vec<Vec<(usize, f64)>> = vec![Vec::new(); cand.len()];
    for (i, r) in rows.iter().enumerate() {
        for &(j, a) in r {
            if is_cand[j] != usize::MAX {
                cols[is_cand[j]].push((i, a));
            }
        }
    }
    let mut used = vec![0.0f64; rows.len()];
    let budget = |i: usize, row_lo: &[f64], row_up: &[f64]| if row_lo[i] == row_up[i] { 1e-9 * (1.0 + row_up[i].abs()) } else { tol };
    let mut fixed = 0;
    for (c, &(j, v, s)) in cand.iter().enumerate() {
        if cols[c].iter().any(|&(i, a)| used[i] + a.abs() * s > budget(i, row_lo, row_up)) {
            continue;
        }
        for &(i, a) in &cols[c] {
            used[i] += a.abs() * s;
        }
        lo[j] = v;
        up[j] = v;
        fixed += 1;
    }
    // 緩める量には前処理の丸めの余裕 1e-9 (1 + |b|) を足す: 右辺 1 - 2e-9 を 1e-9 だけ緩めた 0.9999999989999999 は、整数の行の
    // 正規化 ([`normalize_integer_rows`] の `floor(b + 1e-9)`) の境目のわずかに下で 0 に丸められる (fuzz_tiny の seed 21)
    // `ENOMOTO_MIP_MEM3_TINY_MARGIN` (2026-11-04 から既定、`=0` で以前の 1e-9 (1 + |b|) に戻す): 丸めの余裕を 1e-9 (1 + min(|b|, 1)) + 4 eps |b| にする (右辺の大きい行を
    // 1e-9 |b| (右辺 5590 で 5.6e-6、許容誤差 1e-6 の数倍) 緩めない)。正規化の floor(b + 1e-9) の境目の心配は |b| ≤ 1 の行と同じに残る。
    // uccase12 では再スタートの後の前処理のプロービングがこの余裕で右辺 5000 前後の行を 5.6e-6 ずつ緩め、木の暫定解が元の問題で
    // 連続列の下限 0 を 1.1e-5 破っていた (analysis/uccase12_mem3_20261103.md)
    // 既定 (`m`、2026-11-04) は外側の求解 (再スタートを含む) だけで小さい余裕にし、サブ MIP では以前の余裕のまま (サブ MIP の解は元の問題で
    // 確かめてから使う。qiu はサブ MIP でも小さい余裕にすると系統的に遅くなった (仕組みは未確認)、
    // analysis/halve_submip_20261030.md の bench24)。`=1` でサブ MIP でも小さい余裕
    // 注意: 既定の `ENOMOTO_MIP_SUBMIP_HIGHS` (`p` を含む) ではサブ MIP の前処理にも `submip = false` が渡るので、`m` でもサブ MIP の前処理は小さい余裕になる
    // (qiu の遅れの本当の仕組みは根の解発見スレッドを止める時刻、uccase12 はサブ MIP の小さい余裕にも頼っている。analysis/qiu_margin_20261104.md)
    // `m` のサブ MIP の判定に `SUBMIP_DEPTH` も使う試作 (`ENOMOTO_MIP_QIU_MARGIN_DEPTH`) は uccase12 の下限を破るので削除した
    // (2026-11-18、analysis/qiu_margin_20261104.md・analysis/cleanup_20261118/)
    let mut small = env_def!("ENOMOTO_MIP_MEM3_TINY_MARGIN", "m").is_some_and(|v| v != "m" || !submip);
    // 調査用 (`ENOMOTO_MIP_QIU_MARGIN_SITES`、一時、analysis/qiu_margin_20261104.md): 小さい余裕を使う呼び出し元を並べる (コンマ区切り)。
    // 各項は `o:場所` (外側の求解) か `s:場所` (サブ MIP)、場所は pre0・pre1・probe・quick・restart か `*`。設定すると MEM3_TINY_MARGIN より優先する
    if let Some(v) = env_str!("ENOMOTO_MIP_QIU_MARGIN_SITES") {
        // サブ MIP かは `SUBMIP_DEPTH` でも見る (既定の `ENOMOTO_MIP_SUBMIP_HIGHS` の `p` ではサブ MIP の前処理にも `submip = false` が渡る)
        let who = if submip || heuristics::SUBMIP_DEPTH.with(|d| d.get()) > 0 { "s" } else { "o" };
        small = v.split(',').map(str::trim).any(|t| t == "*" || t == format!("{who}:{site}") || t == format!("{who}:*"));
    }
    let margin = |b: f64| if small { 1e-9 * (1.0 + b.abs().min(1.0)) + 4.0 * f64::EPSILON * b.abs() } else { 1e-9 * (1.0 + b.abs()) };
    let (mut nrel, mut wmax, mut mmax) = (0usize, 0.0f64, 0.0f64);
    for (i, &w) in used.iter().enumerate() {
        if w > 0.0 && row_lo[i] < row_up[i] {
            let (ml, mu) = (margin(row_lo[i]), margin(row_up[i]));
            nrel += 1;
            wmax = wmax.max(w);
            mmax = mmax.max(if row_up[i].is_finite() { mu } else { 0.0 }).max(if row_lo[i].is_finite() { ml } else { 0.0 });
            row_lo[i] -= w + ml;
            row_up[i] += w + mu;
        }
    }
    // 診断用 (`ENOMOTO_MIP_MEM3_DEBUG_INC`、一時): 緩めた行の数、固定でずれた分の最大、丸めの余裕の最大
    if mem3_debug_inc_on() && fixed > 0 {
        let depth = heuristics::SUBMIP_DEPTH.with(|d| d.get());
        eprintln!("MEM3_TINY: site {site} submip {submip} depth {depth}: fixed {fixed} columns; relaxed {nrel} rows, max shift {wmax:.3e}, max margin {mmax:.3e} (small margin {small})");
    }
    fixed
}

/// 係数の強化 (coefficient tightening、SCIP・HiGHS の前処理と同じ): 片側の行 `sum a_j x_j <= b` (`>=` の行は符号を
/// 反転して扱う) の最大活動量 `M` (有限のとき) について、幅 1 の整数列 `x_j` (`[l, l + 1]`) の係数を縮める。
///
/// - `a_j > 0` で `M - a_j < b` (x_j が下限なら行は常に満たされる): `δ = b - (M - a_j)` だけ `a_j` と `b` を減らす。
///   x_j が上限なら他の列への制約は変わらず、下限なら行は `M - a_j <= b - δ` で常に満たされる。
/// - `a_j < 0` で `M + a_j < b` (x_j が上限なら常に満たされる): `δ = b - (M + a_j)` だけ `a_j` を増やす (`b` はそのまま)。
///
/// どちらも整数解の集合を変えず、LP の緩和だけを締める (例: 0-1 列の `sum 21 y >= 2` は `sum 2 y >= 2`)。
/// 列の値は変えないので後処理は要らない。縮めた係数の数を返す。
fn tighten_coefficients(rows: &mut [Vec<(usize, f64)>], row_lo: &mut [f64], row_up: &mut [f64], lo: &[f64], up: &[f64], is_int: &[bool], general: bool) -> usize {
    // 対象の列: 既定は幅 1 の整数列 (0-1 型)。`ENOMOTO_MIP_COEF_TIGHTEN_GENERAL` なら両側の境界が有限な整数列すべて。
    // x_j ∈ [l, u] (整数) で a > 0 なら、x_j <= u - 1 で行が冗長 (M - a <= b) のとき d = b - (M - a) だけ縮め、
    // 右辺を d u 減らす (x_j = u では元と同じ、x_j <= u - 1 では残りの最大活動量で成り立つ)。a < 0 なら右辺を d l 増やす。
    // 幅 1 (0-1 列) はその特別な場合。一般整数まで広げる (`ENOMOTO_MIP_COEF_TIGHTEN_GENERAL`) と gt2 の前処理後の LP が
    // 13460 -> 20146.8 (HiGHS と同じ) になるが、40 問では mik-250-20-75-4 が 2 回とも遅くなり (31.7 -> 56-60 秒) 既定にはしない
    let bin_only = !general;
    let unit = |j: usize| {
        is_int[j]
            && lo[j].is_finite()
            && up[j].is_finite()
            && up[j] - lo[j] >= 1.0 - 1e-9
            && (lo[j] - lo[j].round()).abs() <= 1e-9
            && (up[j] - up[j].round()).abs() <= 1e-9
            && (!bin_only || (up[j] - lo[j] - 1.0).abs() <= 1e-9)
    };
    let mut count = 0;
    for i in 0..rows.len() {
        let (l, u) = (row_lo[i], row_up[i]);
        // 片側の行だけ (等式・範囲制約は両側の条件が絡むので扱わない)
        let sg = if u.is_finite() && !l.is_finite() {
            1.0
        } else if l.is_finite() && !u.is_finite() {
            -1.0
        } else {
            continue;
        };
        if !rows[i].iter().any(|&(j, _)| unit(j)) {
            continue;
        }
        // sg * row <= b
        let mut b = if sg > 0.0 { u } else { -l };
        let mut maxact = 0.0;
        let mut finite = true;
        for &(j, a) in &rows[i] {
            let a = sg * a;
            let v = if a > 0.0 { up[j] } else { lo[j] };
            if !v.is_finite() {
                finite = false;
                break;
            }
            maxact += a * v;
        }
        if !finite {
            continue;
        }
        let tol = 1e-9 * (1.0 + b.abs()).max(maxact.abs());
        // 縮めた後の係数の大きさは `M - b` (残りの最大活動量が右辺を越える分)。これが許容誤差 (`1e-6 (1 + |b|)`) 以下の行は
        // 許容誤差の範囲で冗長なので縮めない (`ENOMOTO_MIP_COEF_TIGHTEN_TINY` で以前どおり縮める)。縮めると、許容誤差の範囲でしか
        // 成り立たない点を厳密に切る行になる: uccase12 の再スタート後の問題では、伝播の誤差の分だけ緩めた幅 1e-9 の連続列 y と
        // 0-1 列 x の行 `x + y <= 1 - 1e-9` (M - b = 2e-9) が `2e-9 x + y <= 1e-9` になり、x = 1 (最適解の値) を切っていた
        // (行の尺度をかけた LP では違反が 1e-9 でなく 0.5 になる)。根の LP が 11507.36 -> 12043.6 と最適値 11507.405 を越え、
        // 誤った最適値を返した (analysis/equiv2_bug_20261022.md)
        let red_tol = if env_str!("ENOMOTO_MIP_COEF_TIGHTEN_TINY").is_some() { 0.0 } else { 1e-6 * (1.0 + b.abs()) };
        let mut changed = false;
        for e in rows[i].iter_mut() {
            let j = e.0;
            if !unit(j) {
                continue;
            }
            let a = sg * e.1;
            if a > 0.0 {
                let d = b - (maxact - a);
                if d > tol && d < a && d >= a - red_tol {
                    COEF_TIGHTEN_SKIPPED.with(|c| c.set(c.get() + 1));
                }
                if d > tol && d < a - red_tol {
                    e.1 = sg * (a - d);
                    // 右辺は d u_j 減る (0-1 型なら u_j = l_j + 1)。最大活動量も x_j の上限での寄与の分 d u_j 減る
                    b -= d * up[j];
                    maxact -= d * up[j];
                    count += 1;
                    changed = true;
                }
            } else if a < 0.0 {
                let d = b - (maxact + a);
                if d > tol && d < -a && d >= -a - red_tol {
                    COEF_TIGHTEN_SKIPPED.with(|c| c.set(c.get() + 1));
                }
                if d > tol && d < -a - red_tol {
                    e.1 = sg * (a + d);
                    // 最大活動量は x_j の下限での寄与 a l_j が (a + d) l_j になる
                    maxact += d * lo[j];
                    b += d * lo[j];
                    count += 1;
                    changed = true;
                }
            }
        }
        if changed {
            if sg > 0.0 {
                row_up[i] = b;
            } else {
                row_lo[i] = -b;
            }
        }
    }
    count
}

/// `ENOMOTO_MIP_FZ2_FIX` (2026-11-22 に既定化、`=0` で切る): 前処理の整数への丸めの余裕を許容誤差 (1e-6) に揃える
/// (analysis/fuzz_tiny_worse_20261103.md、analysis/shelved_20261122)
/// 既定 (未指定) と値 `1` は全部、文字の並びならその部分だけ: `n` 整数行の正規化 ([`normalize_integer_rows`])、`m` 併合した整数列の上限
pub(crate) fn fz2_fix(c: char) -> bool {
    match env_str!("ENOMOTO_MIP_FZ2_FIX") {
        None => true,
        Some(v) => v != "0" && (v == "1" || v.contains(c)),
    }
}

/// 整数列だけで係数が整数の片側の行を係数の最大公約数で割り、右辺を整数に丸める (`<=` は切り下げ、`>=` は切り上げ。
/// 整数解では左辺が整数なので成り立つ)。例: `2 y0 + 2 y1 >= 2` -> `y0 + y1 >= 1`。割った (または丸めた) 行の数を返す。
fn normalize_integer_rows(rows: &mut [Vec<(usize, f64)>], row_lo: &mut [f64], row_up: &mut [f64], is_int: &[bool]) -> usize {
    fn gcd(a: u64, b: u64) -> u64 {
        if b == 0 { a } else { gcd(b, a % b) }
    }
    let mut count = 0;
    for i in 0..rows.len() {
        let (l, u) = (row_lo[i], row_up[i]);
        if l.is_finite() == u.is_finite() || rows[i].is_empty() {
            continue;
        }
        if !rows[i].iter().all(|&(j, a)| is_int[j] && (a - a.round()).abs() <= 1e-9 && a.abs() < 1e9) {
            continue;
        }
        let g = rows[i].iter().fold(0u64, |g, &(_, a)| gcd(g, a.round().abs() as u64)) as f64;
        if g < 1.0 {
            continue;
        }
        // `ENOMOTO_MIP_FZ2_FIX` (既定、`=0` で切る): 丸めの余裕を 1e-9 でなく整数の許容誤差 1e-6 にする。1e-9 だと右辺
        // `1 - 2e-9` が 0 に切り下がり、許容誤差の内 (違反 2e-9) で実行可能な点を厳密に切る (右辺 `1 - 1e-9` は 1 に残るので、
        // 違反 1e-9 は通し 2e-9 は切る不揃いになる。analysis/fuzz_tiny_worse_20261103.md)
        let rt = if fz2_fix('n') { 1e-6 } else { 1e-9 };
        let (nl, nu) = if u.is_finite() { (l, (u / g + rt).floor()) } else { ((l / g - rt).ceil(), u) };
        if g > 1.0 || (u.is_finite() && nu != u) || (l.is_finite() && nl != l) {
            for e in rows[i].iter_mut() {
                e.1 = (e.1 / g).round();
            }
            row_lo[i] = nl;
            row_up[i] = nu;
            count += 1;
        }
    }
    count
}

/// 双対の推論による連続列の境界の締め付け (FJ の問題の前処理 `ENOMOTO_MIP_FJ_P3` で使う): 費用が非負 (最小化) の連続列 x_j が、
/// x_j を増やすと楽になる行 (`<=` の向きで係数が負) では、ほかの列が最悪の値 (その行の最大活動) でも行を満たす値
/// `u* = max_i (maxact_i(j を除く) - b_i) / |a_ij|` を超える必要はない。x_j を増やすと苦しくなる向き (等式を含む) には出ない
/// ときだけ使う。u* より大きい解は x_j を u* に下げても行は破れず目的も悪くならないので、上限を `max(lb, u*)` にできる。
/// 費用が非正なら向きを逆にして下限を締める。physiciansched6-2 の罰則の列 (`100 Σx + 600 z - w <= 600` だけに出る費用 1 の w、
/// 416 本) は ENOMOTO では [0, ∞) のまま残り、HiGHS の前処理後は [0, 100〜600] になる。上限がないと Feasibility Jump が
/// 解を見つけられない (同じ問題で上限を入れるだけで 20 回中 0 回 → 19 回)。締めた境界の数を返す (1 回だけ、連鎖しない)。
/// `ENOMOTO_MIP_SUBMIP_HIGHS` (既定 `-s`、`0` か `off` で使わない (2026-10-31 より前の既定)。analysis/cand_default_20261031.md): サブ MIP の解き方を HiGHS の `HighsPrimalHeuristics::solveSubMip` に寄せる部分 `c`
/// を使うか (サブ MIP の中かどうかは呼び出し側で見る)。値が `1` (など下の文字を含まない値) なら全部、文字の並びならその部分だけ、
/// `-` で始まる文字の並びならその部分を除いた全部 (例 `-t`。`q` はまとめに入らない)。部分: `p` 前処理を親と同じにする、`c` 根のカットを親と同じだけ回す
/// (25 ラウンド・停滞 10 回・時間はサブ MIP の 5 割。部分は親と同じ `ENOMOTO_MIP_CUT_AGG` の値)、`q` サブ MIP の根のカットで集約カットの
/// 枠 (`CUT_AGG` の `q`、50 本) を使う、`s` 擬似費用の信頼の観測を 0 にする (強分岐をしない)、`g` ギャップを親と同じにする、
/// `t` 時間の上限を残り時間にする、`i` RINS の固定率が足りないときの足し方を HiGHS と同じにする、`h` サブ MIP の中でも RENS を回す、
/// `f` 双対の証明・衝突を使う (analysis/submip_20261024.md)
pub(super) fn submip_highs(c: char) -> bool {
    // `q` は文字で明示したときだけ (まとめ (`1`・`-…`) には入れない: physiciansched6-2 の根の RINS の書き出し 3 つで、`q` を足すと
    // 20 秒で解けるのが 3 つ → 1 つに減った)
    const ALL: &str = "pcqsgtihf";
    // `ENOMOTO_MIP_SUBMIP_HIGHS_TRIM`・`ENOMOTO_MIP_SUBMIP_HIGHS_RESTART` で文字を抜いたサブ MIP の中 ([`heuristics::SUBMIP_HS_DROP`])
    if heuristics::SUBMIP_HS_DROP.with(|f| f.get()) & submip_hs_bit(c) != 0 {
        return false;
    }
    match env_def!("ENOMOTO_MIP_SUBMIP_HIGHS", "-s") {
        None => false,
        Some(v) if v.starts_with('-') => c != 'q' && !v.contains(c),
        Some(v) if v.chars().any(|d| ALL.contains(d)) => v.contains(c),
        Some(_) => c != 'q',
    }
}

/// [`submip_highs`] の文字の並び (`pcqsgtihf`) での `c` の位置の印 (知らない文字は 0)。[`heuristics::SUBMIP_HS_DROP`] で使う
pub(super) fn submip_hs_bit(c: char) -> u16 {
    "pcqsgtihf".find(c).map_or(0, |i| 1 << i)
}

/// 文字の並び `v` の [`submip_hs_bit`] の和
pub(super) fn submip_hs_mask(v: &str) -> u16 {
    v.chars().fold(0, |m, c| m | submip_hs_bit(c))
}

/// [`fj_p3_problem`] を使うか (既定で使う。`ENOMOTO_MIP_NO_FJ_P3` で止める。`ENOMOTO_MIP_FJ_P3` を与えたときも使う)。
pub(super) fn fj_p3_on() -> bool {
    env_str!("ENOMOTO_MIP_FJ_P3").is_some() || env_str!("ENOMOTO_MIP_NO_FJ_P3").is_none()
}

pub(super) type FjP3 = (MipProblem, std::sync::Arc<Vec<crate::presolve::PostsolveStep>>, std::sync::Arc<Option<crate::presolve::scaling::Scaling>>);
/// [`fj_p3_problem`] の結果の控え (試作 `ENOMOTO_MIP_FJP3_CACHE`、analysis/objtarget_20261119/ の再スタートの費用): 同じ問題 (行・列・境界・費用の指紋が同じ)
/// なら前処理をかけ直さずに使う。再スタートの後の FJ のスレッドは、最初の回と同じ元の問題 (対称性の行を足す前) から FJ_P3 の問題を作り直していた
/// (nw04: 8.7 万列の前処理に 7.7〜9 秒、その間は止めの合図を見ず、主が再スタートの後の根で 4.8〜6.6 秒待った)。プロセスで 1 つだけ控え、新しい問題が来たら置き換える。
static FJP3_CACHE: std::sync::Mutex<Option<(u64, std::sync::Arc<FjP3>)>> = std::sync::Mutex::new(None);

fn fjp3_key(pc: &MipProblem) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (pc.n, pc.m).hash(&mut h);
    for j in 0..pc.n {
        pc.col_lo[j].to_bits().hash(&mut h);
        pc.col_up[j].to_bits().hash(&mut h);
        pc.cost[j].to_bits().hash(&mut h);
        pc.is_int[j].hash(&mut h);
    }
    for i in 0..pc.m {
        pc.row_lo[i].to_bits().hash(&mut h);
        pc.row_up[i].to_bits().hash(&mut h);
        for &(j, a) in &pc.rows[i] {
            j.hash(&mut h);
            a.to_bits().hash(&mut h);
        }
    }
    h.finish()
}

/// Feasibility Jump だけに渡す問題 (`ENOMOTO_MIP_FJ_P3`、既定で使う ([`fj_p3_on`])): 本体の前処理後の問題 `pc` に、このスレッドだけ
/// 不等式行の列シングルトンと目的を定める連続列の消去 (削除した試作 `ENOMOTO_MIP_INEQ_SINGLETON`・`_CS_OBJ_PIVOT` と同じ処理) を有効にした前処理をもう一度かけ、双対の推論で連続列の境界を締める
/// ([`dual_bound_tightening`])。本体の問題は変えない。前処理は列の番号を保つので、FJ の解は `postsolve` を逆順に当てて尺度を
/// 戻せば `pc` の空間の解になる ([`fj_p3_solution`])。physiciansched6-2 では FJ が解を見つけるのにこの 3 つと FJ の LP 修復の
/// 4 つが全部要る (1 つでも外すと 20 秒で解なし) が、p5s として本体の前処理に入れると beasleyC3・uccase12 で証明が遅れる
pub(super) fn fj_p3_problem(pc: &MipProblem) -> Option<FjP3> {
    let cache = env_str!("ENOMOTO_MIP_FJP3_CACHE").is_some();
    let key = if cache { fjp3_key(pc) } else { 0 };
    if cache {
        if let Some((k, v)) = FJP3_CACHE.lock().ok().and_then(|c| c.clone()) {
            if k == key {
                if env_str!("ENOMOTO_MIP_LOG").is_some() {
                    eprintln!("MIP: FJ problem (FJ_P3): reused the cached presolve (ENOMOTO_MIP_FJP3_CACHE)");
                }
                return Some((v.0.clone(), v.1.clone(), v.2.clone()));
            }
        }
    }
    let r = fj_p3_problem_inner(pc).map(|(q, post, sc)| (q, std::sync::Arc::new(post), std::sync::Arc::new(sc)));
    // 止めの合図・期限で途中で終わった前処理は控えない (同じ問題でも結果が変わりうる)
    if cache && !presolve_stop() {
        if let (Some(v), Ok(mut c)) = (r.as_ref(), FJP3_CACHE.lock()) {
            *c = Some((key, std::sync::Arc::new((v.0.clone(), v.1.clone(), v.2.clone()))));
        }
    }
    r
}

fn fj_p3_problem_inner(pc: &MipProblem) -> Option<(MipProblem, Vec<crate::presolve::PostsolveStep>, Option<crate::presolve::scaling::Scaling>)> {
    let t0 = std::time::Instant::now();
    // 診断: `ENOMOTO_MIP_FJ_DEBUG` と `ENOMOTO_MIP_DEBUG_PRETIME` があれば、FJ_P3 の前処理の段ごとの秒も出す (FJ のスレッドの表)
    let pt = env_str!("ENOMOTO_MIP_FJ_DEBUG").is_some() && pretime::env_on();
    if pt {
        pretime::start();
        eprint!("MIP:   FJ_P3 ");
    }
    crate::set_fj_p3_force(true);
    let r = presolve_mip(pc, false, false, None);
    crate::set_fj_p3_force(false);
    if pt {
        pretime::report(pc.m, pc.n, pc.nnz, 0);
    }
    let (mut prob, post, sc) = match r {
        Some(Presolved::Reduced { prob, postsolve, scaling }) if prob.n == pc.n => (prob, postsolve, Some(scaling)),
        Some(Presolved::Reduced { .. }) | Some(Presolved::Infeasible) => return None,
        None => (pc.clone(), Vec::new(), None),
    };
    let k = dual_bound_tightening(&mut prob);
    if env_str!("ENOMOTO_MIP_FJ_SP_BOUNDS").is_some() {
        let ks = localmip::sp_implied_bounds(&mut prob);
        eprintln!("MIP: FJ problem: {ks} column bounds tightened by set-partitioning rows (FJ_SP_BOUNDS)");
    }
    if env_str!("ENOMOTO_MIP_LOG").is_some() {
        eprintln!("MIP: FJ problem (FJ_P3): rows {} -> {}, free columns {} -> {}, {k} continuous bounds tightened ({:.2}s{})", pc.m, prob.m, (0..pc.n).filter(|&j| pc.col_lo[j] < pc.col_up[j]).count(), (0..prob.n).filter(|&j| prob.col_lo[j] < prob.col_up[j]).count(), t0.elapsed().as_secs_f64(), if presolve_stop() { ", past the deadline" } else { "" });
    }
    Some((prob, post, sc))
}

/// [`fj_p3_problem`] の問題での解 `x` を元の問題 `pc` の空間に戻す。
pub(super) fn fj_p3_solution(x: Vec<f64>, post: &[crate::presolve::PostsolveStep], sc: &Option<crate::presolve::scaling::Scaling>) -> Vec<f64> {
    let mut x = x;
    for step in post.iter().rev() {
        step.apply(&mut x);
    }
    match sc {
        Some(sc) => crate::presolve::scaling::unscale_x(sc, &x),
        None => x,
    }
}

/// 双対の推論による境界の締め付け (`ENOMOTO_MIP_DUAL_BOUND2`、試作。HiGHS の前処理が gt2 の整数列の上限を 9 -> 4 にする縮約):
/// 費用が `c_j >= 0` の列 x_j の上限は、x_j を増やすと楽になる行 (の向き) をすべて、ほかの列の最悪の値でも満たす x_j の値
/// `need` まで下げてよい (x_j > need の解は x_j = need にしても、楽になる行は満たしたまま、苦しくなる行は楽になり、目的は
/// 悪くならない)。下限も向きを逆にして同じ。整数列は `need` を切り上げる。[`dual_bound_tightening`] は苦しくなる行が
/// 1 本でもあるとやめるが、その条件は要らない。締めた境界は次の列の最悪の活動に使う (順に、それまでに締めた問題で正しい)。
/// 締めた境界の数を返す。
#[allow(clippy::too_many_arguments)]
fn dual_bound_tightening2(rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64], cols: &[Vec<(usize, f64)>], cost: &[f64], is_int: &[bool], col_lo: &mut [f64], col_up: &mut [f64]) -> usize {
    let m = rows.len();
    let (mut mn, mut mx) = (vec![0.0f64; m], vec![0.0f64; m]);
    let (mut mn_inf, mut mx_inf) = (vec![0u32; m], vec![0u32; m]);
    let contrib = |a: f64, l: f64, u: f64| if a > 0.0 { (a * l, a * u) } else { (a * u, a * l) };
    for (i, r) in rows.iter().enumerate() {
        for &(j, a) in r {
            let (lo_c, up_c) = contrib(a, col_lo[j], col_up[j]);
            if lo_c.is_finite() { mn[i] += lo_c } else { mn_inf[i] += 1 }
            if up_c.is_finite() { mx[i] += up_c } else { mx_inf[i] += 1 }
        }
    }
    let mut count = 0;
    for j in 0..cols.len() {
        if col_lo[j] >= col_up[j] || cols[j].is_empty() {
            continue;
        }
        for d in [1.0f64, -1.0] {
            if d * cost[j] < 0.0 {
                continue;
            }
            // y = d x_j。cur は y の上限、other は y の下限
            let cur = if d > 0.0 { col_up[j] } else { -col_lo[j] };
            let other = if d > 0.0 { col_lo[j] } else { -col_up[j] };
            if !other.is_finite() {
                continue;
            }
            let mut need = other;
            let mut ok = true;
            'rows: for &(i, a) in &cols[j] {
                for (side, b) in [(1.0f64, row_up[i]), (-1.0, -row_lo[i])] {
                    // side 向きの行: side * row <= b。y の係数 c。c > 0 の向きは y を減らすと楽になるので見なくてよい
                    let c = side * d * a;
                    if !b.is_finite() || c >= 0.0 {
                        continue;
                    }
                    // y を増やすと楽になる向き: c y <= b - (ほかの列の活動)。ほかの列の最悪 = side 向きの最大活動
                    let (act, inf) = if side > 0.0 { (mx[i], mx_inf[i]) } else { (-mn[i], mn_inf[i]) };
                    let ax = side * a;
                    let own = if ax > 0.0 { ax * col_up[j] } else { ax * col_lo[j] };
                    let own_inf = !own.is_finite();
                    if inf - own_inf as u32 > 0 {
                        ok = false;
                        break 'rows;
                    }
                    let rest = if own_inf { act } else { act - own };
                    need = need.max((rest - b) / (-c));
                }
            }
            if !ok || !need.is_finite() {
                continue;
            }
            let nb = if is_int[j] { (need - 1e-9 * (1.0 + need.abs())).ceil().max(other) } else { need + 1e-9 * (1.0 + need.abs()) };
            if nb < cur - 1e-6 * (1.0 + cur.abs().min(1e9)) {
                // 行の活動を新しい境界で更新する
                let (ol, ou) = (col_lo[j], col_up[j]);
                if d > 0.0 { col_up[j] = nb } else { col_lo[j] = -nb }
                for &(i, a) in &cols[j] {
                    let (l0, u0) = contrib(a, ol, ou);
                    let (l1, u1) = contrib(a, col_lo[j], col_up[j]);
                    if l0.is_finite() { mn[i] -= l0 } else { mn_inf[i] -= 1 }
                    if u0.is_finite() { mx[i] -= u0 } else { mx_inf[i] -= 1 }
                    if l1.is_finite() { mn[i] += l1 } else { mn_inf[i] += 1 }
                    if u1.is_finite() { mx[i] += u1 } else { mx_inf[i] += 1 }
                }
                count += 1;
            }
        }
    }
    count
}

fn dual_bound_tightening(prob: &mut MipProblem) -> usize {
    let m = prob.rows.len();
    let (mut mn, mut mx) = (vec![0.0f64; m], vec![0.0f64; m]);
    let (mut mn_inf, mut mx_inf) = (vec![0u32; m], vec![0u32; m]);
    for (i, r) in prob.rows.iter().enumerate() {
        for &(j, a) in r {
            let (l, u) = (prob.col_lo[j], prob.col_up[j]);
            let (lo_c, up_c) = if a > 0.0 { (a * l, a * u) } else { (a * u, a * l) };
            if lo_c.is_finite() { mn[i] += lo_c } else { mn_inf[i] += 1 }
            if up_c.is_finite() { mx[i] += up_c } else { mx_inf[i] += 1 }
        }
    }
    let mut count = 0;
    let cols = prob.cols.clone();
    for j in 0..prob.n {
        if prob.is_int[j] || prob.col_lo[j] == prob.col_up[j] || cols[j].is_empty() {
            continue;
        }
        // d = +1: 上限を締める (費用 >= 0)、d = -1: 下限を締める (費用 <= 0)。費用 0 は両方を試す
        for d in [1.0f64, -1.0] {
            if d * prob.cost[j] < 0.0 {
                continue;
            }
            let cur = if d > 0.0 { prob.col_up[j] } else { -prob.col_lo[j] };
            let other = if d > 0.0 { prob.col_lo[j] } else { -prob.col_up[j] };
            if !other.is_finite() {
                continue;
            }
            // y = d x_j を増やすと楽になる行だけか。楽になる行ごとに必要な y の値の最大
            let mut need = other;
            let mut ok = true;
            for &(i, a) in &cols[j] {
                let ay = d * a; // y の係数
                for (side, b) in [(1.0f64, prob.row_up[i]), (-1.0, -prob.row_lo[i])] {
                    if !b.is_finite() {
                        continue;
                    }
                    // side 向きの行: side * row <= b。y の係数 side * ay
                    let c = side * ay;
                    if c > 0.0 {
                        ok = false; // y を増やすと苦しくなる向きがある
                        break;
                    }
                    // 楽になる向き: c y <= b - (ほかの列の活動)。ほかの列の最悪 = side 向きの最大活動
                    let (act, inf) = if side > 0.0 { (mx[i], mx_inf[i]) } else { (-mn[i], mn_inf[i]) };
                    let own = {
                        let ax = side * a;
                        if ax > 0.0 { ax * prob.col_up[j] } else { ax * prob.col_lo[j] }
                    };
                    let own_inf = !own.is_finite();
                    if inf - own_inf as u32 > 0 {
                        ok = false;
                        break;
                    }
                    let rest = if own_inf { act } else { act - own };
                    // c y <= b - rest かつ c < 0: y >= (rest - b) / |c|
                    need = need.max((rest - b) / (-c));
                }
                if !ok {
                    break;
                }
            }
            if !ok || !need.is_finite() {
                continue;
            }
            // 少し余裕を持たせる (丸め誤差で最適解を落とさない)
            let nb = need + 1e-9 * (1.0 + need.abs());
            if nb < cur - 1e-6 * (1.0 + cur.abs().min(1e9)) {
                if d > 0.0 {
                    prob.col_up[j] = nb;
                } else {
                    prob.col_lo[j] = -nb;
                }
                count += 1;
            }
        }
    }
    count
}

/// 支配される 0-1 列の固定 (`ENOMOTO_MIP_DOMCOL`、2026-11-18 から既定 (`=0` で止める)。HiGHS の前処理の "dominated columns"、Gamrath ら 2015):
/// 0-1 列の文字 (`y = x` か `y = 1 - x`) で考える。文字 y_j・y_k で、費用が `c(y_j) <= c(y_k)`、すべての行で y_j の係数が y_k の
/// 係数より行を破りにくい (上限のある向きで `a_ij <= a_ik`、下限のある向きで `a_ij >= a_ik`、等式は等しい) とき、y_j は y_k を
/// 支配する: y_k = 1 の解から y_k を 0 に、y_j を 1 増やすと、行はどれも破れず目的も悪くならない。よって
/// - y_j と y_k が衝突する (同時に 1 にならない。衝突グラフの辺) なら y_k = 0 の最適解がある
/// - y_j の上限 1 が行から導ける (ほかの列の境界と 1 本の行で `y_j <= 1` が出る。増やしても境界を越えない) なら y_k = 0
/// のどちらかで y_k を 0 に固定する。候補は、衝突グラフの y_j の隣と、(上限が導けるとき) y_j の最も短い行の列。
/// 固定した列を含む組は使わない (固定は順に、それまでの固定をした問題で正しい。支配と「導ける」は境界を締めても崩れない)。
/// 調べる組は `ENOMOTO_T_MIP_DOMCOL_MAX_PAIRS` まで (既定 100 万)。固定 (列, 値) を返す。
/// 診断 `ENOMOTO_MIP_DIAG_COSTSHIFT` (analysis/slowm_20261119 §14): 等式の行 `A_E x = b_E` で費用をずらした `c' = c − λᵀA_E` (実行可能な点では
/// 目的が定数 `λᵀb_E` だけ違うだけ) で [`dominated_binary_fixings_with`] を数える。λ は (r) 根の LP の双対値の等式の分と、(l) LP
/// `min Σ_{j: 0-1 列で等式に出る} max(0, −c'_j)` の解。元の費用の固定の数 `base` と並べて 1 行で出す。
fn diag_costshift(prob: &MipProblem, base: usize) {
    use crate::simplex::mip_lp::TwoStageLp;
    use lp::{LpStatus, SolveLimits};
    let t0 = std::time::Instant::now();
    let n = prob.n;
    let eq: Vec<bool> = (0..prob.m).map(|i| prob.row_lo[i] == prob.row_up[i] && prob.row_lo[i].is_finite()).collect();
    let neq = eq.iter().filter(|&&e| e).count();
    let binary = |j: usize| prob.is_int[j] && prob.col_lo[j] == 0.0 && prob.col_up[j] == 1.0;
    let in_eq: Vec<bool> = (0..n).map(|j| prob.cols[j].iter().any(|&(i, _)| eq[i])).collect();
    let jset: Vec<usize> = (0..n).filter(|&j| binary(j) && in_eq[j]).collect();
    let nfree = (0..n).filter(|&j| prob.col_lo[j] < prob.col_up[j]).count();
    let shifted = |lam: &[f64]| -> Vec<f64> { (0..n).map(|j| prob.cost[j] - prob.cols[j].iter().filter(|&&(i, _)| eq[i]).map(|&(i, a)| lam[i] * a).sum::<f64>()).collect() };
    let count = |c: &[f64]| -> (usize, usize) {
        let mut q = prob.clone();
        q.cost = c.to_vec();
        let neg = jset.iter().filter(|&&j| c[j] < -1e-9).count();
        (dominated_binary_fixings_with(&q, env_def!("ENOMOTO_MIP_DOMCOL_PRED", "1").is_some()).len(), neg)
    };
    let neg0 = jset.iter().filter(|&&j| prob.cost[j] < -1e-9).count();
    // (r) 根の LP の双対値
    let deadline = Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
    let t1 = std::time::Instant::now();
    let mut lp = TwoStageLp::new(&prob.col_lo, &prob.col_up, &prob.cost, &prob.rows, &prob.row_lo, &prob.row_up);
    let st_r = lp.solve(&SolveLimits { deadline, ..Default::default() });
    let it_r = lp.total_iterations();
    let (fix_r, neg_r) = if st_r == LpStatus::Optimal {
        let y = lp.row_duals();
        let lam: Vec<f64> = (0..prob.m).map(|i| if eq[i] { y[i] } else { 0.0 }).collect();
        count(&shifted(&lam))
    } else {
        (usize::MAX, usize::MAX)
    };
    let secs_r = t1.elapsed().as_secs_f64();
    // (l) λ を LP で選ぶ: 変数 λ_i (等式 i、自由) と t_j ≥ 0 (j ∈ jset)、行 t_j − Σ_i a_ij λ_i ≥ −c_j、min Σ t_j
    let t2 = std::time::Instant::now();
    let eqi: Vec<usize> = (0..prob.m).filter(|&i| eq[i]).collect();
    let mut pos = vec![usize::MAX; prob.m];
    for (k, &i) in eqi.iter().enumerate() {
        pos[i] = k;
    }
    let ne = eqi.len();
    let nv = ne + jset.len();
    let mut lo = vec![f64::NEG_INFINITY; nv];
    let up = vec![f64::INFINITY; nv];
    let mut cc = vec![0.0; nv];
    for t in 0..jset.len() {
        lo[ne + t] = 0.0;
        cc[ne + t] = 1.0;
    }
    let mut rows = Vec::with_capacity(jset.len());
    let (mut rlo, mut rup) = (Vec::with_capacity(jset.len()), Vec::with_capacity(jset.len()));
    for (t, &j) in jset.iter().enumerate() {
        let mut r: Vec<(usize, f64)> = prob.cols[j].iter().filter(|&&(i, _)| eq[i]).map(|&(i, a)| (pos[i], -a)).collect();
        r.push((ne + t, 1.0));
        r.sort_unstable_by_key(|e| e.0);
        rows.push(r);
        rlo.push(-prob.cost[j]);
        rup.push(f64::INFINITY);
    }
    let (mut fix_l, mut neg_l, mut it_l, mut st_l) = (usize::MAX, usize::MAX, 0usize, LpStatus::Optimal);
    if ne > 0 && !jset.is_empty() {
        let mut lp2 = TwoStageLp::new(&lo, &up, &cc, &rows, &rlo, &rup);
        st_l = lp2.solve(&SolveLimits { deadline: Some(std::time::Instant::now() + std::time::Duration::from_secs(60)), ..Default::default() });
        it_l = lp2.total_iterations() as usize;
        if st_l == LpStatus::Optimal {
            let v = lp2.col_values();
            let mut lam = vec![0.0; prob.m];
            for (k, &i) in eqi.iter().enumerate() {
                lam[i] = v[k];
            }
            (fix_l, neg_l) = count(&shifted(&lam));
        }
    }
    let secs_l = t2.elapsed().as_secs_f64();
    eprintln!(
        "COSTSHIFT {{\"n\":{n},\"free\":{nfree},\"m\":{},\"eq\":{neq},\"bin_in_eq\":{},\"base\":{base},\"neg0\":{neg0},\"root\":{{\"st\":\"{st_r:?}\",\"iters\":{it_r},\"fix\":{},\"neg\":{},\"secs\":{secs_r:.3}}},\"lam\":{{\"st\":\"{st_l:?}\",\"iters\":{it_l},\"fix\":{},\"neg\":{},\"secs\":{secs_l:.3}}},\"secs\":{:.3}}}",
        prob.m, jset.len(), fix_r as i64, neg_r as i64, fix_l as i64, neg_l as i64, t0.elapsed().as_secs_f64()
    );
}

fn dominated_binary_fixings(prob: &MipProblem) -> Vec<(usize, f64)> {
    // 試作 `ENOMOTO_MIP_PREGD` の `p` ([`identical_col_fixings`]): 同じ列で費用の違う列を先に固定し、固定した問題で 0-1 列の支配を調べる
    if pregd('p') {
        let ident = identical_col_fixings(prob);
        if !ident.is_empty() {
            let mut q = prob.clone();
            for &(j, v) in &ident {
                q.col_lo[j] = v;
                q.col_up[j] = v;
            }
            let mut out = ident;
            out.extend(dominated_binary_fixings_with(&q, env_def!("ENOMOTO_MIP_DOMCOL_PRED", "1").is_some()));
            return out;
        }
    }
    dominated_binary_fixings_with(prob, env_def!("ENOMOTO_MIP_DOMCOL_PRED", "1").is_some())
}

/// 試作 `ENOMOTO_MIP_PREGD` (既定では使わない。analysis/pregd_20261128) の形の文字 `c` が入っているか
fn pregd(c: char) -> bool {
    env_str!("ENOMOTO_MIP_PREGD").is_some_and(|v| v.contains(c))
}

/// `ENOMOTO_MIP_PREGD` の `p` (analysis/pregd_20261128): 係数が全く同じ列 (同じ型) の組で、費用の安い列 x_j が高い列 x_k を支配する
/// (HiGHS の dominatedColumns の、等式の行では係数が等しくなければならない場合。nw04 は 87482 列のうち 41292 列がこれで、HiGHS は 36777 列を固定する)。
/// x_k を下限から δ 上げた解は、x_k を下限に戻して x_j を δ 上げても行の活動は同じで、目的は (c_k − c_j) δ ≥ 0 だけ良くなる。x_j を上げられるのは
/// (a) x_j の上限が無いとき、か (b) 両方 0-1 で同時に 1 にできない (同じ列なので、ある行で 2 本とも 1 にすると最小活動でも破れる) とき。
/// どちらかなら x_k を下限に固定する (同じ組の固定はみな x_j に移せる: (a) は上限が無く、(b) は組の中で 1 になれるのは 1 本だけ)。
/// x_j は固定しない。返り値は (列, 値)。
fn identical_col_fixings(prob: &MipProblem) -> Vec<(usize, f64)> {
    let n = prob.n;
    let m = prob.rows.len();
    // 行の最小・最大活動 (無限の項の数つき)
    let (mut mn, mut mx) = (vec![0.0f64; m], vec![0.0f64; m]);
    let (mut mn_inf, mut mx_inf) = (vec![0u32; m], vec![0u32; m]);
    for (i, r) in prob.rows.iter().enumerate() {
        for &(j, a) in r {
            let (l, u) = (prob.col_lo[j], prob.col_up[j]);
            let (lo_c, up_c) = if a > 0.0 { (a * l, a * u) } else { (a * u, a * l) };
            if lo_c.is_finite() { mn[i] += lo_c } else { mn_inf[i] += 1 }
            if up_c.is_finite() { mx[i] += up_c } else { mx_inf[i] += 1 }
        }
    }
    let binary = |j: usize| prob.is_int[j] && prob.col_lo[j] == 0.0 && prob.col_up[j] == 1.0;
    let mut groups: std::collections::HashMap<(bool, Vec<(usize, u64)>), Vec<usize>> = std::collections::HashMap::new();
    for j in 0..n {
        if prob.col_lo[j] >= prob.col_up[j] || prob.cols[j].is_empty() {
            continue;
        }
        let mut key: Vec<(usize, u64)> = prob.cols[j].iter().map(|&(i, a)| (i, (a + 0.0).to_bits())).collect();
        key.sort_unstable();
        groups.entry((prob.is_int[j], key)).or_default().push(j);
    }
    let mut out = Vec::new();
    for mut g in groups.into_values() {
        if g.len() < 2 {
            continue;
        }
        g.sort_by(|&a, &b| prob.cost[a].total_cmp(&prob.cost[b]).then(a.cmp(&b)));
        let j = g[0];
        // (b): 組の 0-1 列のどれか 2 本を 1 にすると破れる行があるか (組の列は 0-1 で下限 0 なので、最小活動に組の列は入っていない)
        let conflict = g.iter().all(|&k| binary(k))
            && prob.cols[j].iter().any(|&(i, a)| {
                let tol = 1e-9 * (1.0 + prob.row_up[i].abs().min(1e12).max(prob.row_lo[i].abs().min(1e12)));
                (a > 0.0 && prob.row_up[i].is_finite() && mn_inf[i] == 0 && mn[i] + 2.0 * a > prob.row_up[i] + tol)
                    || (a < 0.0 && prob.row_lo[i].is_finite() && mx_inf[i] == 0 && mx[i] + 2.0 * a < prob.row_lo[i] - tol)
            });
        let up_free = prob.col_up[j] == f64::INFINITY;
        if !conflict && !up_free {
            continue;
        }
        for &k in &g[1..] {
            if prob.col_lo[k].is_finite() {
                out.push((k, prob.col_lo[k]));
            }
        }
    }
    out
}

/// [`dominated_binary_fixings`] の本体。`pred` は `ENOMOTO_MIP_DOMCOL_PRED` (予測の境界の (c) も使う)。
fn dominated_binary_fixings_with(prob: &MipProblem, pred: bool) -> Vec<(usize, f64)> {
    let n = prob.n;
    let binary: Vec<bool> = (0..n).map(|j| prob.is_int[j] && prob.col_lo[j] == 0.0 && prob.col_up[j] == 1.0).collect();
    // 試作 `ENOMOTO_MIP_PREGD` の `g` (analysis/pregd_20261128): 0-1 列に限らず、両側に有限の境界を持つ列 (一般の整数・連続) も調べる。
    // 文字は y = x − l (s = +1) か y = u − x (s = −1) で、幅は w = u − l。支配の補題 (y_j が y_k を支配すれば、y_j = w_j か y_k = 0 の
    // 最適解がある: y_k を下げた分を y_j に移す) は、移す量が y_j の型に合う限り型によらない。整数の y_j が連続の y_k を支配することは
    // 使わない (HiGHS の dominatedColumns と同じ)。衝突グラフ (候補 (a)) は 0-1 列だけ。
    let gen = pregd('g');
    let elig: Vec<bool> = (0..n).map(|j| binary[j] || (gen && prob.col_lo[j] < prob.col_up[j] && prob.col_lo[j].is_finite() && prob.col_up[j].is_finite())).collect();
    // 文字 (j, s) が 0 (top = false) か w (top = true) のときの x の値
    let x_at = |j: usize, s: f64, top: bool| if (s > 0.0) == top { prob.col_up[j] } else { prob.col_lo[j] };
    let g = clique::CliqueGraph::build_with(n, &prob.rows, &prob.row_lo, &prob.row_up, &binary, &prob.col_lo, &prob.col_up, &prob.conflict_edges);
    let max_pairs = tunable!("ENOMOTO_T_MIP_DOMCOL_MAX_PAIRS", 1_000_000usize, usize);
    let mut cols: Vec<Vec<(usize, f64)>> = prob.cols.clone();
    for c in cols.iter_mut() {
        c.sort_unstable_by_key(|e| e.0);
    }
    // 行ごとの最小・最大活動 (有限部分と無限の項の数)
    let m = prob.rows.len();
    let (mut mn, mut mx) = (vec![0.0f64; m], vec![0.0f64; m]);
    let (mut mn_inf, mut mx_inf) = (vec![0u32; m], vec![0u32; m]);
    for (i, r) in prob.rows.iter().enumerate() {
        for &(j, a) in r {
            let (l, u) = (prob.col_lo[j], prob.col_up[j]);
            let (lo_c, up_c) = if a > 0.0 { (a * l, a * u) } else { (a * u, a * l) };
            if lo_c.is_finite() { mn[i] += lo_c } else { mn_inf[i] += 1 }
            if up_c.is_finite() { mx[i] += up_c } else { mx_inf[i] += 1 }
        }
    }
    // 文字 (j, s) の上限 1 が行から導けるか (s = +1: x_j <= 1、s = -1: x_j >= 0)。0-1 列の寄与は有限
    let implied_up = |j: usize, s: f64| -> bool {
        cols[j].iter().any(|&(i, a)| {
            // 向き d (上限側 +1: a x <= up、下限側 -1: -a x <= -lo) ごとに、文字の係数が正になる向きで
            [(1.0, prob.row_up[i]), (-1.0, -prob.row_lo[i])].into_iter().any(|(d, b)| {
                let al = d * a * s; // 文字 y の係数 (x = y か 1 - y)
                if !b.is_finite() || al <= 0.0 {
                    return false;
                }
                // d 向きの最小活動 (j を除く)
                let (act, inf) = if d > 0.0 { (mn[i], mn_inf[i]) } else { (-mx[i], mx_inf[i]) };
                if inf > 0 {
                    return false;
                }
                let aj = d * a;
                let own = if aj > 0.0 { aj * prob.col_lo[j] } else { aj * prob.col_up[j] };
                let rest = act - own;
                // 文字 y の項: aj x = aj x(y = 0) + al y (定数 aj x(y = 0) は右辺へ。0-1 列なら s = +1 で 0、s = -1 で aj)
                let bb = b - aj * x_at(j, s, false);
                (bb - rest) / al <= (prob.col_up[j] - prob.col_lo[j]) + 1e-9
            })
        })
    };
    // 文字 (j, sj) が (k, sk) を支配するか
    let dominates = |j: usize, sj: f64, k: usize, sk: f64| -> bool {
        if prob.is_int[j] && !prob.is_int[k] {
            return false;
        }
        if sj * prob.cost[j] > sk * prob.cost[k] + 1e-9 * (1.0 + prob.cost[k].abs()) {
            return false;
        }
        let (cj, ck) = (&cols[j], &cols[k]);
        let (mut a, mut b) = (0, 0);
        while a < cj.len() || b < ck.len() {
            let (i, aj, ak) = if b >= ck.len() || (a < cj.len() && cj[a].0 < ck[b].0) {
                a += 1;
                (cj[a - 1].0, sj * cj[a - 1].1, 0.0)
            } else if a >= cj.len() || ck[b].0 < cj[a].0 {
                b += 1;
                (ck[b - 1].0, 0.0, sk * ck[b - 1].1)
            } else {
                a += 1;
                b += 1;
                (cj[a - 1].0, sj * cj[a - 1].1, sk * ck[b - 1].1)
            };
            let tol = 1e-9 * (1.0 + aj.abs().max(ak.abs()));
            if prob.row_up[i].is_finite() && aj > ak + tol {
                return false;
            }
            if prob.row_lo[i].is_finite() && aj < ak - tol {
                return false;
            }
        }
        true
    };
    // `ENOMOTO_MIP_DOMCOL_PRED` (2026-11-18 から既定、`=0` で止める): HiGHS の予測の境界 (Gamrath ら 2015 の定理 3 (vi)) に当たる (c) も使う: y_j が y_k を支配し
    // c(y_k) >= 0 のとき、y_j = 1・y_k = 0 にして、y_k を下げると破れうる行 (y_k の係数が上限側で負・下限側で正) が、ほかの
    // 列の最悪の値でも満たされるなら y_k = 0 にできる (支配の補題で y_j = 1 か y_k = 0 の最適解があり、y_j = 1 の側でも
    // y_k を 0 に下げて行は破れず目的も悪くならない)。physiciansched6-2 には係数も費用も同じ 0-1 列が 3 本ずつの組が 13 あり、
    // HiGHS は各組の 2 本をこれで 0 に固定する (3 本の和が 1 を超えても得をしない)
    // y_j = w_j・y_k = 0 にしたときに、y_k を下げると破れうる行がほかの列の最悪の値でも満たされるか
    let worst_case_ok = |j: usize, sj: f64, k: usize, sk: f64| -> bool {
        if sk * prob.cost[k] < -1e-9 * (1.0 + prob.cost[k].abs()) {
            return false;
        }
        let (xj, xk) = (x_at(j, sj, true), x_at(k, sk, false));
        cols[k].iter().all(|&(i, ak)| {
            // y_k を下げると (x_k の向き -sk) 行の活動は -sk * ak だけ動く
            let dk = -sk * ak;
            let aj = cols[j].binary_search_by_key(&i, |e| e.0).map(|p| cols[j][p].1).unwrap_or(0.0);
            // j・k を除いたほかの列の最大・最小活動
            let contrib = |jj: usize, a: f64| if a > 0.0 { (a * prob.col_lo[jj], a * prob.col_up[jj]) } else { (a * prob.col_up[jj], a * prob.col_lo[jj]) };
            let (jl, ju) = contrib(j, aj);
            let (kl, ku) = contrib(k, ak);
            let fixed_act = aj * xj + ak * xk;
            // 上限側: 活動が増える向き (dk > 0) なら最大活動で確かめる
            if prob.row_up[i].is_finite() && dk > 0.0 {
                if mx_inf[i] > 0 {
                    return false;
                }
                if mx[i] - ju - ku + fixed_act > prob.row_up[i] + 1e-9 * (1.0 + prob.row_up[i].abs()) {
                    return false;
                }
            }
            if prob.row_lo[i].is_finite() && dk < 0.0 {
                if mn_inf[i] > 0 {
                    return false;
                }
                if mn[i] - jl - kl + fixed_act < prob.row_lo[i] - 1e-9 * (1.0 + prob.row_lo[i].abs()) {
                    return false;
                }
            }
            true
        })
    };
    let mut fixed = vec![false; n];
    let mut out = Vec::new();
    let mut pairs = 0usize;
    let (mut pred_pairs, max_pred_pairs) = (0usize, tunable!("ENOMOTO_T_MIP_DOMCOL_PRED_MAX_PAIRS", 1_000_000usize, usize));
    let mut cand: Vec<(usize, f64, bool)> = Vec::new(); // (k, sk, 衝突しているか)
    'outer: for j in 0..n {
        if !elig[j] {
            continue;
        }
        for sj in [1.0, -1.0] {
            if fixed[j] {
                continue 'outer;
            }
            let lj = if sj > 0.0 { 2 * j as u32 } else { 2 * j as u32 + 1 };
            cand.clear();
            for &lk in g.neighbors(lj) {
                cand.push(((lk / 2) as usize, if lk % 2 == 0 { 1.0 } else { -1.0 }, true));
            }
            // (b) の候補 (y_j の最も短い行)
            if implied_up(j, sj) {
                if let Some(&(i0, _)) = cols[j].iter().min_by_key(|&&(i, _)| prob.rows[i].len()) {
                    for &(k, _) in &prob.rows[i0] {
                        cand.push((k, 1.0, false));
                        cand.push((k, -1.0, false));
                    }
                }
            }
            // (c) の候補: HiGHS の bestRowPlus (文字 y_j の係数が行を締める向き (上限側で正、上限がなく下限側で負) の最も
            // 短い行)。y_j が y_k を支配するなら y_k はその行にある。(a)(b) の候補の後に、別の上限
            // `ENOMOTO_T_MIP_DOMCOL_PRED_MAX_PAIRS` (既定 100 万) の中で調べる (同じ上限にすると長い行で (a)(b) の手間を使い切る。
            // blp-ar98 で固定が 1539 → 16 になった)
            let n_ab = cand.len();
            if pred && pred_pairs < max_pred_pairs {
                let best = cols[j].iter().filter(|&&(i, a)| (if prob.row_up[i].is_finite() { 1.0 } else { -1.0 }) * a * sj > 0.0).min_by_key(|&&(i, _)| prob.rows[i].len());
                if let Some(&(i0, _)) = best {
                    for &(k, _) in &prob.rows[i0] {
                        cand.push((k, 1.0, false));
                        cand.push((k, -1.0, false));
                    }
                }
            }
            let up_j = pred && implied_up(j, sj);
            for (ci, &(k, sk, conf)) in cand.iter().enumerate() {
                if k == j || !elig[k] || fixed[k] {
                    continue;
                }
                if ci < n_ab {
                    pairs += 1;
                    if pairs > max_pairs {
                        break 'outer;
                    }
                } else {
                    pred_pairs += 1;
                    if pred_pairs > max_pred_pairs {
                        break;
                    }
                }
                if dominates(j, sj, k, sk) && (!pred || conf || up_j || worst_case_ok(j, sj, k, sk)) {
                    // y_k = 0: sk = +1 なら x_k = 0、sk = -1 なら x_k = 1
                    fixed[k] = true;
                    out.push((k, x_at(k, sk, false)));
                }
            }
        }
    }
    // `ENOMOTO_MIP_DOMCOL_REV` (2026-11-18 から既定、`=0` で止める。`ENOMOTO_MIP_DOMCOL_PRED` と併せて働く): 支配される側 y_k から候補を探す。上の (c) は
    // 支配する側 y_j の最も短い行から y_k を探すが、y_j が y_k を支配するなら、y_k が行を「助ける」向き (下限側で係数が正、
    // 上限側で負) の行には必ず y_j も (同じ向きにより大きく) 入るので、y_k のそういう行で最も短いものから y_j を探せば漏れがない。
    // 集合被覆 (rail507・fast0507。Σ x ≥ 1 の行だけ) では y_j の列が y_k の列を含むのが支配で、上の探し方では候補がほとんど
    // 出ない (rail507 で 26 本、上限を外しても 9428 本・13 秒。SCIP の domcol は 42278 本・0.6 秒)。
    // 固定は (c) と同じく worst_case_ok (か y_j の上限 1 が行から導ける) のときだけ。y_j に固定済みの列は使わない。
    // 調べる組は `ENOMOTO_T_MIP_DOMCOL_REV_MAX_PAIRS` まで (既定 5000 万)。
    if pred && env_def!("ENOMOTO_MIP_DOMCOL_REV", "1").is_some() {
        let max_rev = tunable!("ENOMOTO_T_MIP_DOMCOL_REV_MAX_PAIRS", 50_000_000usize, usize);
        let mut rev_pairs = 0usize;
        'rev: for k in 0..n {
            if !elig[k] {
                continue;
            }
            for sk in [1.0, -1.0] {
                if fixed[k] {
                    continue 'rev;
                }
                // 文字 y_k の費用が負なら worst_case_ok が通らないので飛ばす
                if sk * prob.cost[k] < -1e-9 * (1.0 + prob.cost[k].abs()) {
                    continue;
                }
                let best = cols[k]
                    .iter()
                    .filter(|&&(i, a)| (prob.row_lo[i].is_finite() && a * sk > 0.0) || (prob.row_up[i].is_finite() && a * sk < 0.0))
                    .min_by_key(|&&(i, _)| prob.rows[i].len());
                let Some(&(i0, _)) = best else { continue };
                for &(j, _) in &prob.rows[i0] {
                    if j == k || !elig[j] || fixed[j] {
                        continue;
                    }
                    for sj in [1.0, -1.0] {
                        rev_pairs += 1;
                        if rev_pairs > max_rev {
                            break 'rev;
                        }
                        if dominates(j, sj, k, sk) && (worst_case_ok(j, sj, k, sk) || implied_up(j, sj)) {
                            fixed[k] = true;
                            out.push((k, x_at(k, sk, false)));
                            continue 'rev;
                        }
                    }
                }
            }
        }
    }
    out
}

/// 行の疎化 (`ENOMOTO_MIP_SPARSIFY`、既定では使わない。HiGHS の前処理の Sparsify の、フィルインのない半分): 等式の行 `e`
/// (2 列以上) の列がすべて行 `r` に含まれるとき、`r` から `s e` (`s` は消せる列の最も多い倍率) を引いて非零を減らす。
/// 等式の倍数を足し引きするだけなので実行可能領域は変わらず、後処理も要らない。整数列だけの行で係数が整数なら `s` も整数に限る
/// (係数を整数のまま保つ。係数の強化・カットの前提)。書き換えた行は、この呼び出しでは等式として使わず、等式として使った行は
/// 書き換えない (相互に引き合うと系が壊れる)。1 行から複数の等式を引いてよい。手間は 1 本の等式あたり、その最も出現の少ない
/// 列を含む行の数まで。
///
/// physiciansched6-2 では、勤務の数の長い行 (77 列の `Σ ≤ 26`) から同じ列の部分集合の等式 (66 列の `Σ = 18`) を引くと
/// 11 列の `Σ ≤ 8` になる (66 列は互いに素な 18 本の等式 `Σ = 1`)。HiGHS の前処理は 4 本しか書き換えないが、これを切ると
/// Feasibility Jump が解を見つけられない。
/// 書き換えた行の数を返す。
fn sparsify_rows(rows: &mut [Vec<(usize, f64)>], row_lo: &mut [f64], row_up: &mut [f64], is_int: &[bool], n: usize) -> usize {
    let m = rows.len();
    let max_len = tunable!("ENOMOTO_T_MIP_SPARSIFY_MAX_EQ_LEN", 1000usize, usize);
    let mut col_rows: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, r) in rows.iter().enumerate() {
        for &(j, _) in r {
            col_rows[j].push(i);
        }
    }
    // 長い等式から (長い行ほど減らせる非零が多い)
    let mut eqs: Vec<usize> = (0..m).filter(|&i| row_lo[i] == row_up[i] && rows[i].len() >= 2 && rows[i].len() <= max_len).collect();
    eqs.sort_by_key(|&i| std::cmp::Reverse(rows[i].len()));
    let mut changed = vec![false; m];
    let mut pivot = vec![false; m];
    let mut count = 0usize;
    for e in eqs {
        if changed[e] {
            continue;
        }
        let re = rows[e].clone();
        let be = row_lo[e];
        let Some(&(anchor, _)) = re.iter().min_by_key(|&&(j, _)| col_rows[j].len()) else { continue };
        let cand = col_rows[anchor].clone();
        for r in cand {
            if r == e || pivot[r] || rows[r].len() <= re.len() {
                continue;
            }
            let rr = &rows[r];
            // r の係数 (列は昇順でないこともあるので探す表を作る)
            let coef: std::collections::HashMap<usize, f64> = rr.iter().copied().collect();
            if !re.iter().all(|(j, _)| coef.contains_key(j)) {
                continue;
            }
            // 倍率の候補: 各列の a_r / a_e。最も多く出るもの
            let mut ratios: Vec<f64> = re.iter().map(|&(j, a)| coef[&j] / a).collect();
            ratios.sort_by(|a, b| a.total_cmp(b));
            let (mut best, mut best_cnt) = (0.0, 0usize);
            let mut k = 0;
            while k < ratios.len() {
                let mut l = k;
                while l < ratios.len() && (ratios[l] - ratios[k]).abs() <= 1e-12 * (1.0 + ratios[k].abs()) {
                    l += 1;
                }
                if l - k > best_cnt {
                    best_cnt = l - k;
                    best = ratios[k];
                }
                k = l;
            }
            if best_cnt == 0 || best == 0.0 {
                continue;
            }
            // 整数列だけで係数が整数の行は、倍率も整数に限る
            let int_row = rr.iter().all(|&(j, a)| is_int[j] && a == a.round());
            if int_row && (best != best.round() || !re.iter().all(|&(_, a)| a == a.round())) {
                continue;
            }
            let ecoef: std::collections::HashMap<usize, f64> = re.iter().copied().collect();
            let mut nr: Vec<(usize, f64)> = Vec::with_capacity(rr.len());
            for &(j, a) in rr {
                let v = match ecoef.get(&j) {
                    Some(&ae) => {
                        let v = a - best * ae;
                        if v.abs() <= 1e-12 * (1.0 + a.abs()) { 0.0 } else { v }
                    }
                    None => a,
                };
                if v != 0.0 {
                    nr.push((j, v));
                }
            }
            if nr.len() >= rr.len() {
                continue;
            }
            let shift = best * be;
            if env_str!("ENOMOTO_MIP_DEBUG_SPARSIFY").is_some() {
                eprintln!("MIP:   sparsify row {r} (len {} -> {}, [{}, {}]) by equation {e} (len {}, = {be}) x {best}", rr.len(), nr.len(), row_lo[r], row_up[r], re.len());
            }
            if row_lo[r].is_finite() {
                row_lo[r] -= shift;
            }
            if row_up[r].is_finite() {
                row_up[r] -= shift;
            }
            rows[r] = nr;
            if !changed[r] {
                count += 1;
            }
            changed[r] = true;
            pivot[e] = true;
        }
    }
    count
}

/// 行の疎化を HiGHS 1.15.1 の `HPresolve::sparsify` と同じ選び方でする (`ENOMOTO_MIP_SPARSIFY=highs`、既定では使わない)。
/// [`sparsify_rows`] との違い:
/// - 等式は **短い順** (HiGHS の `equations` は (行の長さ, 行) の順の集合で、呼び出しの初めの写しを順に使う)。書き換えた等式も
///   その後に (その時の中身で) 等式として使う
/// - 候補の行は等式の最も出現の少ない列を含む行。フィルインを 1 列まで許す (等式が連続列だけで候補の行に整数列があれば 0。
///   一般整数列 (幅 1.5 超) のフィルインは許さない)。加えて、最も出現の少ない列が一般整数でなければ、2 番目に少ない列を含み
///   最も少ない列を含まない行 (等式のほかの列はすべて含む) も候補にする (等式が連続列だけなら整数列のある行は除く)
/// - 倍率は `|s| <= 1e3` で、消える非零の最も多いもの (同数なら |s| の小さい方)。倍率どうしが `sqrt(1e-6)/|a|` より近いときの
///   扱いも HiGHS と同じ。消える数がフィルインより多ければ書き換える。整数の倍率に限らない
/// - 書き換えで絶対値 1e-9 以下になった係数は消す (`small_matrix_value`)
/// 書き換えた行の数を返す。
fn sparsify_rows_highs(rows: &mut [Vec<(usize, f64)>], row_lo: &mut [f64], row_up: &mut [f64], is_int: &[bool], col_lo: &[f64], col_up: &[f64], n: usize) -> usize {
    use std::collections::{BTreeSet, HashMap};
    let m = rows.len();
    let small = 1e-9;
    let min_nz = 1e-6f64.sqrt();
    let general = |j: usize| is_int[j] && col_up[j] - col_lo[j] > 1.5;
    let mut r: Vec<HashMap<usize, f64>> = rows.iter().map(|row| row.iter().copied().filter(|&(_, a)| a != 0.0).collect()).collect();
    let mut col_rows: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
    for (i, row) in r.iter().enumerate() {
        for &j in row.keys() {
            col_rows[j].insert(i);
        }
    }
    let nint = |row: &HashMap<usize, f64>| row.keys().filter(|&&j| is_int[j]).count();
    let mut eqs: Vec<usize> = (0..m).filter(|&i| row_lo[i] == row_up[i] && r[i].len() >= 2).collect();
    eqs.sort_by_key(|&i| (r[i].len(), i));
    let mut changed = vec![false; m];
    let debug = env_str!("ENOMOTO_MIP_DEBUG_SPARSIFY").is_some();
    // possibleScales: 倍率の昇順の (倍率, 数)。数 -1 は数値的に悪い倍率
    let add_scale = |ps: &mut Vec<(f64, i64)>, scale: f64, eqv: f64, candv: f64| {
        let tol = min_nz / eqv.abs();
        let pos = ps.partition_point(|&(s, _)| s < scale - tol);
        if pos < ps.len() && (ps[pos].0 - scale).abs() <= tol {
            if ps[pos].1 == -1 {
                return;
            }
            if (ps[pos].0 * eqv + candv).abs() <= small {
                ps[pos].1 += 1;
            } else {
                ps[pos].1 = -1;
            }
        } else {
            ps.insert(pos, (scale, 1));
        }
    };
    let best_scale = |ps: &[(f64, i64)], misses: i64| -> (f64, i64) {
        let (mut scale, mut num) = (0.0f64, 0i64);
        for &(s, c) in ps {
            if c <= misses {
                continue;
            }
            if c > num || (c == num && s.abs() < scale.abs()) {
                scale = s;
                num = c;
            }
        }
        (scale, num)
    };
    let mut count = 0usize;
    for e in eqs {
        if r[e].len() < 2 || row_lo[e] != row_up[e] {
            continue;
        }
        let mut re: Vec<(usize, f64)> = r[e].iter().map(|(&j, &a)| (j, a)).collect();
        re.sort_unstable_by_key(|x| x.0);
        let rhs = row_lo[e];
        let eq_int = re.iter().filter(|x| is_int[x.0]).count();
        // 最も出現の少ない列と 2 番目
        let (mut c1, mut c2) = (usize::MAX, usize::MAX);
        for &(j, _) in &re {
            if c1 == usize::MAX || col_rows[j].len() < col_rows[c1].len() {
                c2 = c1;
                c1 = j;
            } else if c2 == usize::MAX || col_rows[j].len() < col_rows[c2].len() {
                c2 = j;
            }
        }
        let mut targets: Vec<(usize, f64)> = Vec::new();
        let mut ps: Vec<(f64, i64)> = Vec::new();
        for &cand in &col_rows[c1] {
            if cand == e {
                continue;
            }
            ps.clear();
            let cr = &r[cand];
            let mut misses = 0i64;
            let max_misses = if eq_int == 0 && nint(cr) != 0 { 0 } else { 1 };
            for &(j, a) in &re {
                let Some(&cv) = cr.get(&j) else {
                    if general(j) {
                        misses = 2;
                        break;
                    }
                    misses += 1;
                    if misses > max_misses {
                        break;
                    }
                    continue;
                };
                let scale = -cv / a;
                if scale.abs() > 1e3 {
                    continue;
                }
                add_scale(&mut ps, scale, a, cv);
            }
            if misses > max_misses || ps.is_empty() {
                continue;
            }
            let (scale, num) = best_scale(&ps, misses);
            if num > misses {
                targets.push((cand, scale));
            }
        }
        if !general(c1) && c2 != usize::MAX {
            for &cand in &col_rows[c2] {
                if cand == e || r[cand].contains_key(&c1) {
                    continue;
                }
                let cr = &r[cand];
                if eq_int == 0 && nint(cr) != 0 {
                    continue;
                }
                ps.clear();
                let mut skip = false;
                for &(j, a) in &re {
                    if j == c1 {
                        continue;
                    }
                    let Some(&cv) = cr.get(&j) else {
                        skip = true;
                        break;
                    };
                    let scale = -cv / a;
                    if scale.abs() > 1e3 {
                        continue;
                    }
                    add_scale(&mut ps, scale, a, cv);
                }
                if skip || ps.is_empty() {
                    continue;
                }
                let (scale, num) = best_scale(&ps, 1);
                if num > 1 {
                    targets.push((cand, scale));
                }
            }
        }
        for (row, scale) in targets {
            if debug {
                eprintln!("MIP:   sparsify(highs) row {row} (len {}, [{}, {}]) by equation {e} (len {}, = {rhs}) x {scale}", r[row].len(), row_lo[row], row_up[row], re.len());
            }
            if row_lo[row].is_finite() {
                row_lo[row] += scale * rhs;
            }
            if row_up[row].is_finite() {
                row_up[row] += scale * rhs;
            }
            for &(j, a) in &re {
                let v = r[row].get(&j).copied().unwrap_or(0.0) + scale * a;
                if v.abs() <= small {
                    if r[row].remove(&j).is_some() {
                        col_rows[j].remove(&row);
                    }
                } else if r[row].insert(j, v).is_none() {
                    col_rows[j].insert(row);
                }
            }
            if !changed[row] {
                count += 1;
                changed[row] = true;
            }
        }
    }
    for (i, row) in r.into_iter().enumerate() {
        if changed[i] {
            let mut v: Vec<(usize, f64)> = row.into_iter().collect();
            v.sort_unstable_by_key(|x| x.0);
            rows[i] = v;
        }
    }
    count
}

/// 係数が定数倍の関係にある行をまとめる。各行を最初の係数で割った形で分類し、同じ形の行の
/// `base x` の範囲の共通部分を、最初の行 (元の倍率) の範囲制約 1 本にする。共通部分が空 (実行不能) なら
/// その組はまとめない (伝播・LP が検出する)。まとめた (消した) 行の数を返す。
fn merge_parallel_rows(rows: &mut Vec<Vec<(usize, f64)>>, row_lo: &mut Vec<f64>, row_up: &mut Vec<f64>) -> usize {
    use std::collections::HashMap;
    let m = rows.len();
    let key_of = |r: &[(usize, f64)]| -> Option<(Vec<(usize, i64)>, f64)> {
        let s = r.first()?.1;
        if s == 0.0 {
            return None;
        }
        let mut k: Vec<(usize, i64)> = r.iter().map(|&(j, a)| (j, ((a / s) * 1e12).round() as i64)).collect();
        k.sort_unstable();
        Some((k, s))
    };
    // 形 -> (最初の行, その倍率, base x の下限, 上限)
    let mut groups: HashMap<Vec<(usize, i64)>, (usize, f64, f64, f64)> = HashMap::new();
    let mut keep = vec![true; m];
    let mut merged = 0usize;
    for i in 0..m {
        let Some((k, s)) = key_of(&rows[i]) else { continue };
        // row_lo <= s (base x) <= row_up -> base x の範囲
        let (bl, bu) = if s > 0.0 { (row_lo[i] / s, row_up[i] / s) } else { (row_up[i] / s, row_lo[i] / s) };
        match groups.get_mut(&k) {
            None => {
                groups.insert(k, (i, s, bl, bu));
            }
            Some(g) => {
                let (nl, nu) = (g.2.max(bl), g.3.min(bu));
                if nl > nu + 1e-9 * (1.0 + nl.abs().max(nu.abs())) {
                    continue; // 実行不能な組はまとめない
                }
                g.2 = nl;
                g.3 = nu;
                keep[i] = false;
                merged += 1;
            }
        }
    }
    if merged == 0 {
        return 0;
    }
    for (_, (i, s, bl, bu)) in groups {
        let (lo, up) = if s > 0.0 { (bl * s, bu * s) } else { (bu * s, bl * s) };
        row_lo[i] = lo;
        row_up[i] = up;
    }
    let mut k = 0;
    for i in 0..m {
        if keep[i] {
            rows.swap(k, i);
            row_lo.swap(k, i);
            row_up.swap(k, i);
            k += 1;
        }
    }
    rows.truncate(k);
    row_lo.truncate(k);
    row_up.truncate(k);
    merged
}

#[cfg(test)]
mod tests {
    /// 利用者の打ち切り値より良くない解は返さない ([`patch_user_cutoff`]、analysis/ti6_20261124 の再現: misc07 に打ち切り値 1000 を与えると
    /// 3670 を「最適」と返していた)。最適で終わっていれば「実行不能 (打ち切り値より良い解はない)」、時間切れなら解なし、良い解はそのまま。
    #[test]
    fn user_cutoff_drops_solutions_not_better_than_it() {
        let r = |status, z: Option<f64>| solver::MipResult { status, x: z.map(|_| vec![1.0]), objective: z, best_bound: 1425.0, nodes: 0, lp_iterations: 0 };
        let a = patch_user_cutoff(r(MipStatus::Optimal, Some(3670.0)), 1000.0, false);
        assert_eq!(a.status, MipStatus::Infeasible);
        assert!(a.x.is_none() && a.objective.is_none() && a.best_bound == f64::INFINITY);
        let b = patch_user_cutoff(r(MipStatus::TimeLimit, Some(41160.0)), 28290.02829, false);
        assert_eq!(b.status, MipStatus::TimeLimit);
        assert!(b.x.is_none() && b.objective.is_none() && b.best_bound == 1425.0);
        // 打ち切り値と同じ値 (余裕の内) も返さない
        let c = patch_user_cutoff(r(MipStatus::Optimal, Some(1000.0)), 1000.0, false);
        assert_eq!(c.status, MipStatus::Infeasible);
        // 打ち切り値より良い解・打ち切り値なし (既定) はそのまま
        let d = patch_user_cutoff(r(MipStatus::Optimal, Some(2810.0)), 2810.5, false);
        assert_eq!(d.status, MipStatus::Optimal);
        assert_eq!(d.objective, Some(2810.0));
        let e = patch_user_cutoff(r(MipStatus::Optimal, Some(3670.0)), f64::INFINITY, false);
        assert_eq!(e.status, MipStatus::Optimal);
        assert_eq!(e.objective, Some(3670.0));
    }

    use super::*;

    /// TREE2_FORCEQ ([`tree2_forceq_rows`]): 本当に強制される行 (`x_a + x_b = 1000 y`、x_a・x_b ∈ [0, 500]) は 2 項等式を作り、
    /// 上限との差が小さいだけの行 (x_b の上限 1.002e-6、unitcal_7 の DUALEQ の後の行) は作らない (作ると y = 1, x_b = 0 の点を切る)
    #[test]
    fn tree2_forceq_rejects_near_forcing_rows() {
        let rows = vec![vec![(0, -1000.0), (1, 1.0), (2, 1.0)]];
        let is_int = vec![true, false, false];
        let eqs = tree2_forceq_rows(&rows, &[0.0], &[0.0], &[0.0, 0.0, 0.0], &[1.0, 500.0, 500.0], &is_int);
        assert_eq!(eqs.len(), 2);
        assert!(eqs.iter().all(|(r, rhs)| *rhs == 0.0 && r.contains(&(0, -500.0))));
        let eqs = tree2_forceq_rows(&rows, &[0.0], &[0.0], &[0.0, 0.0, 0.0], &[1.0, 1000.0, 1.001999987900842e-06], &is_int);
        assert!(eqs.is_empty(), "{eqs:?}");
    }

    /// 後処理した解を境界に収める ([`postsolve_clip`]): 費用の大きい列が許容誤差の中で下限 0 を下回る (-1.2e-8) と目的値が
    /// 0.012 下がる (binkar10_1)。収めると目的値は正しい値に戻る。収めると行を許容誤差を越えて破る解は収めない
    #[test]
    fn postsolve_clip_moves_columns_into_bounds() {
        // x0 + x1 = 1、x1 は費用 1e6 の罰金の列 (下限 0)
        let p = MipProblem::from_rows(vec![0.0, 0.0], vec![1.0, f64::INFINITY], vec![1.0, 1e6], 0.0, 1.0, vec![false, false], vec![vec![(0, 1.0), (1, 1.0)]], vec![1.0], vec![1.0]);
        let mut x = vec![1.0 + 1.2e-8, -1.2e-8];
        assert!(p.is_feasible(&x, 1e-6));
        assert!(p.objective(&x) < 1.0 - 0.01);
        postsolve_clip(&p, &mut x, false);
        assert_eq!(x, vec![1.0, 0.0]);
        assert!((p.objective(&x) - 1.0).abs() < 1e-12);
        // 収めると行が 1.5e-6 破れる (許容誤差 1e-6 を越える): 収めない
        let p2 = MipProblem::from_rows(vec![0.0, 0.0], vec![2.0, f64::INFINITY], vec![1.0, 1e6], 0.0, 1.0, vec![false, false], vec![vec![(0, 1.0), (1, 1.0)]], vec![1.0], vec![1.0]);
        let mut y = vec![1.0 + 1.5e-6, -5e-7];
        assert!(p2.is_feasible(&y, 1e-6));
        assert!(!p2.is_feasible(&[1.0 + 1.5e-6, 0.0], 1e-6));
        let y0 = y.clone();
        postsolve_clip(&p2, &mut y, false);
        assert_eq!(y, y0);
    }

    /// クリーク行の拡張: `x0 + x1 <= 1` に、x0・x1 の両方と衝突する x2 (`x0 + x2 <= 1`・`x1 + x2 <= 1`) を加え、
    /// 拡張した行に含まれる 2 本の行は消す。x3 は x1 とだけ衝突するので加えない。否定の文字 (`x4 - x5 <= 0` は
    /// `x4 + (1 - x5) <= 1`) も扱う
    #[test]
    fn extend_clique_rows_adds_common_neighbors() {
        let rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, 1.0)], vec![(1, 1.0), (2, 1.0)], vec![(1, 1.0), (3, 1.0)], vec![(4, 1.0), (5, -1.0)], vec![(4, 1.0), (6, 1.0)], vec![(5, -1.0), (6, 1.0)]];
        let m = rows.len();
        let p = MipProblem::from_rows(vec![0.0; 7], vec![1.0; 7], vec![-1.0; 7], 0.0, 1.0, vec![true; 7], rows, vec![f64::NEG_INFINITY; m], vec![1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 0.0]);
        let q = extend_clique_rows(p, false);
        let has = |r: &[(usize, f64)], up: f64| (0..q.m).any(|i| q.rows[i] == r && q.row_up[i] == up);
        assert!(has(&[(0, 1.0), (1, 1.0), (2, 1.0)], 1.0));
        assert!(has(&[(1, 1.0), (3, 1.0)], 1.0));
        // x4 + (1 - x5) + x6 <= 1  <=>  x4 - x5 + x6 <= 0
        assert!(has(&[(4, 1.0), (5, -1.0), (6, 1.0)], 0.0));
        assert_eq!(q.m, 3);
        // 整数解は変わらない: 元の行を満たす点は拡張後も満たす (全列挙)
        for mask in 0u32..128 {
            let x: Vec<f64> = (0..7).map(|j| ((mask >> j) & 1) as f64).collect();
            let orig_ok = [x[0] + x[1], x[0] + x[2], x[1] + x[2], x[1] + x[3], x[4] + x[6]].iter().all(|&v| v <= 1.0) && x[4] - x[5] <= 0.0 && x[6] - x[5] <= 0.0;
            assert_eq!(orig_ok, q.is_feasible(&x, 1e-9), "{x:?}");
        }
    }

    /// `ENOMOTO_MIP_CLQEXT_KNAP` の本体: ナップサック行から取り出すクリーク行は、元の行を満たすどの 0-1 点でも満たされる (全列挙)
    #[test]
    fn knapsack_clique_rows_are_valid() {
        // 5 x0 + 4 x1 + 4 x2 + 2 x3 + x4 <= 7 (x0..x2 がクリーク)、3 x0 - 3 x5 + 3 x6 + x3 >= -1 (<= の向きで -3 x0 + 3 x5 - 3 x6 - x3 <= 1)
        let rows = vec![vec![(0, 5.0), (1, 4.0), (2, 4.0), (3, 2.0), (4, 1.0)], vec![(0, 3.0), (3, 1.0), (5, -3.0), (6, 3.0)]];
        let (lo, up) = (vec![f64::NEG_INFINITY, -1.0], vec![7.0, f64::INFINITY]);
        let bin = vec![true; 7];
        let out = knapsack_clique_rows(&rows, &lo, &up, &bin, &[0.0; 7], &[1.0; 7]);
        assert!(out.iter().any(|(r, u)| r == &[(0, 1.0), (1, 1.0), (2, 1.0)] && *u == 1.0), "{out:?}");
        assert!(!out.is_empty());
        for mask in 0u32..128 {
            let x: Vec<f64> = (0..7).map(|j| ((mask >> j) & 1) as f64).collect();
            let act = |r: &[(usize, f64)]| r.iter().map(|&(j, a)| a * x[j]).sum::<f64>();
            let ok = (0..2).all(|i| act(&rows[i]) >= lo[i] - 1e-9 && act(&rows[i]) <= up[i] + 1e-9);
            if ok {
                for (r, u) in &out {
                    assert!(act(r) <= u + 1e-9, "{x:?} {r:?} {u}");
                }
            }
        }
    }

    /// クリーク行の拡張は、このスレッドの止めの合図 ([`presolve_stop`]) が立っていれば何も変えずに戻る
    /// (木の従のスレッドのサブ MIP の前処理が、主の止めの合図の後も回り続けないように)
    #[test]
    fn extend_clique_rows_stops_on_halt() {
        let rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, 1.0)], vec![(1, 1.0), (2, 1.0)]];
        let m = rows.len();
        let p = MipProblem::from_rows(vec![0.0; 3], vec![1.0; 3], vec![-1.0; 3], 0.0, 1.0, vec![true; 3], rows, vec![f64::NEG_INFINITY; m], vec![1.0; m]);
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let q = crate::cancel::with_halt(Some(flag), || extend_clique_rows(p.clone(), false));
        assert_eq!(q.rows, p.rows);
        assert_eq!(q.m, 3);
        // 合図がなければ拡張する
        assert_eq!(extend_clique_rows(p, false).m, 1);
    }

    /// 上限側と下限側 (符号の逆の倍) の行が範囲制約 1 本にまとまること
    #[test]
    fn tighten_coefficients_saturates_cover_row() {
        // sum 21 y >= 2 (0-1) -> 2 y0 + 2 y1 + 2 y2 >= 2
        let mut rows = vec![vec![(0, 21.0), (1, 21.0), (2, 21.0)]];
        let (mut lo, mut up) = (vec![2.0], vec![f64::INFINITY]);
        let n = tighten_coefficients(&mut rows, &mut lo, &mut up, &[0.0; 3], &[1.0; 3], &[true; 3], false);
        assert_eq!(n, 3);
        assert_eq!(rows[0], vec![(0, 2.0), (1, 2.0), (2, 2.0)]);
        assert_eq!(lo[0], 2.0);
        // 最大公約数で割って y0 + y1 + y2 >= 1
        assert_eq!(normalize_integer_rows(&mut rows, &mut lo, &mut up, &[true; 3]), 1);
        assert_eq!(rows[0], vec![(0, 1.0), (1, 1.0), (2, 1.0)]);
        assert_eq!(lo[0], 1.0);
        // 3 x0 + 6 x1 <= 7 -> x0 + 2 x1 <= 2
        let mut r2 = vec![vec![(0, 3.0), (1, 6.0)]];
        let (mut l2, mut u2) = (vec![f64::NEG_INFINITY], vec![7.0]);
        assert_eq!(normalize_integer_rows(&mut r2, &mut l2, &mut u2, &[true; 2]), 1);
        assert_eq!(r2[0], vec![(0, 1.0), (1, 2.0)]);
        assert_eq!(u2[0], 2.0);
    }

    /// 許容誤差の範囲で冗長な行 (縮めた後の係数 `M - b` が許容誤差以下) は縮めない。uccase12 の再スタート後の問題の
    /// `x + y <= 1 - 1e-9` (x は 0-1、y は幅 1e-9 の連続列) を `2e-9 x + y <= 1e-9` にすると、x = 1 を厳密に切る
    #[test]
    fn tighten_coefficients_skips_rows_redundant_within_tolerance() {
        let mut rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let (mut lo, mut up) = (vec![f64::NEG_INFINITY], vec![1.0 - 1e-9]);
        let n = tighten_coefficients(&mut rows, &mut lo, &mut up, &[0.0, 0.0], &[1.0, 1e-9], &[true, false], false);
        assert_eq!(n, 0);
        assert_eq!(rows[0], vec![(0, 1.0), (1, 1.0)]);
        assert_eq!(up[0], 1.0 - 1e-9);
        // `>=` の向き (符号を反転した行) も同じ
        let mut rows = vec![vec![(0, -1.0), (1, -1.0)]];
        let (mut lo, mut up) = (vec![-1.0 + 1e-9], vec![f64::INFINITY]);
        assert_eq!(tighten_coefficients(&mut rows, &mut lo, &mut up, &[0.0, 0.0], &[1.0, 1e-9], &[true, false], false), 0);
        // 越える分が許容誤差より大きければ以前どおり縮める: x + 0.25 y <= 1 (M - b = 0.25) -> 0.25 x + 0.25 y <= 0.25
        let mut rows = vec![vec![(0, 1.0), (1, 0.25)]];
        let (mut lo, mut up) = (vec![f64::NEG_INFINITY], vec![1.0]);
        assert_eq!(tighten_coefficients(&mut rows, &mut lo, &mut up, &[0.0, 0.0], &[1.0, 1.0], &[true, false], false), 1);
        assert_eq!(rows[0], vec![(0, 0.25), (1, 0.25)]);
        assert_eq!(up[0], 0.25);
    }

    /// 乱数の片側の行 (幅 1 の整数列と有界な連続列 1 本) で、係数の強化の前後で整数列の値の組ごとの
    /// 実行可能性 (連続列を最も有利な値に置いたとき) が変わらないことを総当たりで確かめる。
    #[test]
    fn tighten_coefficients_keeps_integer_feasible_set() {
        let mut seed = 99u64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut total = 0;
        for _ in 0..3000 {
            let nb = 1 + (rnd() * 5.0) as usize;
            let n = nb + 1;
            let mut lo = vec![0.0; n];
            let mut up = vec![1.0; n];
            let mut is_int = vec![true; n];
            // 整数列の幅は 1-3 (0-1 型と一般整数)
            for j in 0..nb {
                lo[j] = (rnd() * 3.0).floor() - 1.0;
                up[j] = lo[j] + 1.0 + (rnd() * 3.0).floor();
            }
            is_int[nb] = false;
            lo[nb] = -2.0 * rnd();
            up[nb] = lo[nb] + 3.0 * rnd();
            let row: Vec<(usize, f64)> = (0..n).map(|j| (j, ((rnd() - 0.4) * 20.0).round())).filter(|&(_, a)| a != 0.0).collect();
            if row.is_empty() {
                continue;
            }
            let ge = rnd() < 0.5;
            let rhs = ((rnd() - 0.5) * 30.0).round();
            let (rl, ru) = if ge { (rhs, f64::INFINITY) } else { (f64::NEG_INFINITY, rhs) };
            let mut rows = vec![row.clone()];
            let (mut rlo, mut rup) = (vec![rl], vec![ru]);
            total += tighten_coefficients(&mut rows, &mut rlo, &mut rup, &lo, &up, &is_int, true);
            let feasible = |r: &[(usize, f64)], l: f64, u: f64, y: &[f64]| -> bool {
                // 連続列は [lo, up] の好きな値を取れる: 活動量の取りうる範囲が [l, u] と交わるか
                let mut a0 = 0.0;
                let mut ac = 0.0;
                for &(j, a) in r {
                    if j < nb {
                        a0 += a * y[j];
                    } else {
                        ac = a;
                    }
                }
                let (c1, c2) = (a0 + ac * lo[nb], a0 + ac * up[nb]);
                let (mn, mx) = (c1.min(c2), c1.max(c2));
                mx >= l - 1e-9 && mn <= u + 1e-9
            };
            // 整数列のすべての値の組 (各列 up - lo + 1 通り) を数え上げる
            let widths: Vec<usize> = (0..nb).map(|j| (up[j] - lo[j]) as usize + 1).collect();
            let combos: usize = widths.iter().product();
            for mut code in 0..combos {
                let y: Vec<f64> = (0..nb)
                    .map(|j| {
                        let v = lo[j] + (code % widths[j]) as f64;
                        code /= widths[j];
                        v
                    })
                    .collect();
                assert_eq!(feasible(&row, rl, ru, &y), feasible(&rows[0], rlo[0], rup[0], &y), "row {row:?} [{rl}, {ru}] -> {:?} [{}, {}], y {y:?}", rows[0], rlo[0], rup[0]);
            }
        }
        assert!(total > 100, "only {total} coefficients tightened");
    }

    #[test]
    fn sparsify_rows_subtracts_disjoint_equations() {
        // x0 + x1 = 1、x2 + x3 = 1 を x0 + x1 + x2 + x3 + x4 <= 3 から引くと x4 <= 1。等式の行は書き換えない
        let mut rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(2, 1.0), (3, 1.0)], vec![(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0), (4, 1.0)], vec![(0, 2.0), (1, 3.0), (5, 1.0)]];
        let mut lo = vec![1.0, 1.0, f64::NEG_INFINITY, f64::NEG_INFINITY];
        let mut up = vec![1.0, 1.0, 3.0, 4.0];
        let k = sparsify_rows(&mut rows, &mut lo, &mut up, &[true; 6], 6);
        assert_eq!(rows[2], vec![(4, 1.0)]);
        assert_eq!(up[2], 1.0);
        assert_eq!(rows[0].len() + rows[1].len(), 4);
        // 2 x0 + 3 x1 + x5 <= 4: 倍率 2 で x0 を、3 で x1 を消せる (どちらも 1 列)。整数の倍率で 1 列減る
        assert_eq!(rows[3].len(), 2);
        assert_eq!(k, 2);
        // 整数の行で整数でない倍率は使わない: 2 x0 + 2 x1 + x5 <= 3 と 2 x0 + 2 x1 = 2 (倍率 1) は使い、
        // x0 + x1 + x5 <= 1 と 2 x0 + 2 x1 = 2 (倍率 1/2) は使わない
        let mut rows = vec![vec![(0, 2.0), (1, 2.0)], vec![(0, 1.0), (1, 1.0), (5, 1.0)]];
        let mut lo = vec![2.0, f64::NEG_INFINITY];
        let mut up = vec![2.0, 1.0];
        assert_eq!(sparsify_rows(&mut rows, &mut lo, &mut up, &[true; 6], 6), 0);
    }

    /// 支配による固定は最適値を変えない (小さな 0-1 問題を全部数えて比べる)
    #[test]
    fn dominated_binary_fixings_keep_optimum() {
        let mut seed = 12345u64;
        let mut rnd = |k: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % k
        };
        let mut total_fixed = [0usize; 2];
        for _ in 0..2000 {
            let n = 6;
            let m = 1 + rnd(4) as usize;
            let mut rows = Vec::new();
            let (mut rlo, mut rup) = (Vec::new(), Vec::new());
            for _ in 0..m {
                let mut r: Vec<(usize, f64)> = Vec::new();
                for j in 0..n {
                    let (take, a) = (rnd(2) == 0, rnd(5) as f64 - 2.0);
                    if take && a != 0.0 {
                        r.push((j, a));
                    }
                }
                if r.len() < 2 {
                    r = vec![(0, 1.0), (1 + rnd(5) as usize, 1.0)];
                }
                let b = rnd(3) as f64;
                match rnd(3) {
                    0 => {
                        rlo.push(f64::NEG_INFINITY);
                        rup.push(b);
                    }
                    1 => {
                        rlo.push(b - 1.0);
                        rup.push(f64::INFINITY);
                    }
                    _ => {
                        rlo.push(b - 1.0);
                        rup.push(b + 1.0);
                    }
                }
                rows.push(r);
            }
            // 衝突を作るための集合詰め込み行
            rows.push(vec![(rnd(3) as usize, 1.0), (3 + rnd(3) as usize, 1.0)]);
            rlo.push(f64::NEG_INFINITY);
            rup.push(1.0);
            let cost: Vec<f64> = (0..n).map(|_| rnd(5) as f64 - 2.0).collect();
            let p = MipProblem::from_rows(vec![0.0; n], vec![1.0; n], cost, 0.0, 1.0, vec![true; n], rows, rlo, rup);
            let best = |lo: &[f64], up: &[f64]| -> f64 {
                let mut z = f64::INFINITY;
                for mask in 0..(1u32 << n) {
                    let x: Vec<f64> = (0..n).map(|j| ((mask >> j) & 1) as f64).collect();
                    if (0..n).all(|j| x[j] >= lo[j] && x[j] <= up[j]) && p.is_feasible(&x, 1e-9) {
                        z = z.min(p.objective(&x));
                    }
                }
                z
            };
            for pred in [false, true] {
                let fix = dominated_binary_fixings_with(&p, pred);
                let (mut lo, mut up) = (p.col_lo.clone(), p.col_up.clone());
                for &(j, v) in &fix {
                    lo[j] = v;
                    up[j] = v;
                }
                total_fixed[pred as usize] += fix.len();
                assert_eq!(best(&p.col_lo, &p.col_up), best(&lo, &up), "fixings {fix:?} (pred {pred}) change the optimum: rows {:?} lo {:?} up {:?} cost {:?}", p.rows, p.row_lo, p.row_up, p.cost);
            }
        }
        // 予測の境界 (pred) は固定を増やす
        assert!(total_fixed[0] > 0 && total_fixed[1] > total_fixed[0], "{total_fixed:?}");
    }

    /// 双対の推論による連続列の上限 ([`dual_bound_tightening`]) は最適解を落とさない: 0-1 列 x と罰則の連続列 w
    /// (`a x - w <= b`、費用は正) の問題で、総当たりの最適解 (w = max(0, a x - b) の最小) が締めた境界の中にある
    /// 双対の推論による境界 ([`dual_bound_tightening2`]、整数列も) は最適値を変えない: 列 4 本 (整数 0..3)、
    /// 片側・両側の行のある小さな問題で、締めた箱の総当たりの最適値が元の箱の最適値と同じ
    #[test]
    fn dual_bound_tightening2_keeps_optimum() {
        let mut seed = 4242u64;
        let mut rnd = |k: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % k
        };
        let mut tightened = 0;
        for _ in 0..3000 {
            let n = 4;
            let m = 1 + rnd(3) as usize;
            let (mut rows, mut rlo, mut rup) = (Vec::new(), Vec::new(), Vec::new());
            for _ in 0..m {
                let r: Vec<(usize, f64)> = (0..n).filter_map(|j| { let a = rnd(7) as f64 - 3.0; (a != 0.0).then_some((j, a)) }).collect();
                if r.is_empty() {
                    continue;
                }
                let b = rnd(9) as f64 - 2.0;
                match rnd(3) {
                    0 => { rlo.push(f64::NEG_INFINITY); rup.push(b); }
                    1 => { rlo.push(b); rup.push(f64::INFINITY); }
                    _ => { rlo.push(b); rup.push(b + rnd(3) as f64); }
                }
                rows.push(r);
            }
            let cost: Vec<f64> = (0..n).map(|_| rnd(5) as f64 - 2.0).collect();
            let p = MipProblem::from_rows(vec![0.0; n], vec![3.0; n], cost, 0.0, 1.0, vec![true; n], rows, rlo, rup);
            let best = |lo: &[f64], up: &[f64]| -> f64 {
                let mut z = f64::INFINITY;
                for code in 0..4u32.pow(n as u32) {
                    let x: Vec<f64> = (0..n).map(|j| ((code / 4u32.pow(j as u32)) % 4) as f64).collect();
                    if (0..n).all(|j| x[j] >= lo[j] && x[j] <= up[j]) && p.is_feasible(&x, 1e-9) {
                        z = z.min(p.objective(&x));
                    }
                }
                z
            };
            let (mut lo, mut up) = (p.col_lo.clone(), p.col_up.clone());
            tightened += dual_bound_tightening2(&p.rows, &p.row_lo, &p.row_up, &p.cols, &p.cost, &p.is_int, &mut lo, &mut up);
            assert_eq!(best(&p.col_lo, &p.col_up), best(&lo, &up), "bounds {lo:?} {up:?} change the optimum: rows {:?} lo {:?} up {:?} cost {:?}", p.rows, p.row_lo, p.row_up, p.cost);
        }
        assert!(tightened > 0);
    }

    #[test]
    fn dual_bound_tightening_keeps_optimum() {
        let mut seed = 777u64;
        let mut rnd = |k: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % k
        };
        let mut tightened = 0;
        for _ in 0..500 {
            let (nb, nw) = (5usize, 2usize);
            let n = nb + nw;
            let mut rows = Vec::new();
            let (mut rlo, mut rup) = (Vec::new(), Vec::new());
            for w in 0..nw {
                // 罰則の行: Σ a x - w <= b (w を増やすと楽になる)
                let mut r: Vec<(usize, f64)> = Vec::new();
                for j in 0..nb {
                    if rnd(2) == 0 {
                        let a = 1.0 + rnd(4) as f64;
                        r.push((j, a));
                    }
                }
                r.push((nb + w, -1.0));
                rows.push(r);
                rlo.push(f64::NEG_INFINITY);
                rup.push(rnd(4) as f64);
            }
            // 0-1 列だけの行 (被覆・詰め込み)
            let mut cover: std::collections::BTreeMap<usize, f64> = std::collections::BTreeMap::new();
            cover.insert(0, 1.0);
            for j in 0..nb {
                if rnd(2) == 0 {
                    cover.insert(j, 1.0);
                }
            }
            rows.push(cover.into_iter().collect());
            rlo.push(1.0);
            rup.push(f64::INFINITY);
            let cost: Vec<f64> = (0..n).map(|j| if j < nb { rnd(5) as f64 - 2.0 } else { 1.0 + rnd(3) as f64 }).collect();
            let is_int: Vec<bool> = (0..n).map(|j| j < nb).collect();
            let up0: Vec<f64> = (0..n).map(|j| if j < nb { 1.0 } else { f64::INFINITY }).collect();
            let p = MipProblem::from_rows(vec![0.0; n], up0, cost, 0.0, 1.0, is_int, rows, rlo, rup);
            let mut q = p.clone();
            tightened += dual_bound_tightening(&mut q);
            // 総当たり: x を決めると最適な w は各罰則の行の max(0, a x - b)
            let (mut zbest, mut xbest) = (f64::INFINITY, Vec::new());
            for mask in 0..(1u32 << nb) {
                let mut x: Vec<f64> = (0..nb).map(|j| ((mask >> j) & 1) as f64).collect();
                for w in 0..nw {
                    let act: f64 = p.rows[w].iter().filter(|e| e.0 < nb).map(|&(j, a)| a * x[j]).sum();
                    x.push((act - p.row_up[w]).max(0.0));
                }
                if p.is_feasible(&x, 1e-9) && p.objective(&x) < zbest {
                    zbest = p.objective(&x);
                    xbest = x;
                }
            }
            if xbest.is_empty() {
                continue;
            }
            assert!(q.is_feasible(&xbest, 1e-9) && (0..n).all(|j| xbest[j] >= q.col_lo[j] - 1e-9 && xbest[j] <= q.col_up[j] + 1e-9), "tightened bounds cut off the optimum {xbest:?}: {:?} {:?}", q.col_lo, q.col_up);
        }
        assert!(tightened > 0);
    }

    #[test]
    fn merge_parallel_rows_makes_ranged_row() {
        let mut rows = vec![vec![(0, 1.0), (1, 2.0)], vec![(0, -2.0), (1, -4.0)], vec![(0, 1.0), (1, 1.0)]];
        let mut lo = vec![f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        let mut up = vec![5.0, -2.0, 3.0];
        // x0 + 2 x1 <= 5 と -2 x0 - 4 x1 <= -2 (x0 + 2 x1 >= 1)
        let k = merge_parallel_rows(&mut rows, &mut lo, &mut up);
        assert_eq!(k, 1);
        assert_eq!(rows.len(), 2);
        let i = rows.iter().position(|r| r[1].1 == 2.0).unwrap();
        assert!((lo[i] - 1.0).abs() < 1e-12 && (up[i] - 5.0).abs() < 1e-12);
    }

    /// 整数列の整数でない境界は問題を作るときに丸められること (伝播の走査省略やカットは整数の境界を前提にする。
    /// 丸めないと不正なカットが出て最適解を切っていた)。
    #[test]
    fn integer_bounds_are_rounded() {
        let p = MipProblem::from_rows(vec![-0.5, 0.0], vec![1.9665, 2.5], vec![1.0, 1.0], 0.0, 1.0, vec![true, false], vec![vec![(0, 1.0), (1, 1.0)]], vec![f64::NEG_INFINITY], vec![3.0]);
        assert_eq!((p.col_lo[0], p.col_up[0]), (0.0, 1.0));
        assert_eq!((p.col_lo[1], p.col_up[1]), (0.0, 2.5));
    }

    /// 幅 1e-9 の連続列とクリーク型の行 (右辺 1 - 1e-9) を持つ小さな MIP (benchmarks/equiv2_bug_20261022/fuzz_tiny.py の seed 41)。
    /// 最適値 -10.9 の解 (x4 = x5 = x8 = 1、ほかは 0) は厳密に実行可能だが、許容誤差の範囲で冗長な行の係数を縮めていた
    /// ときは前処理がこれを切り、-9.81 を最適と返していた
    #[test]
    fn tiny_range_columns_keep_exactly_feasible_optimum() {
        let inf = f64::INFINITY;
        let mut up = vec![1.0; 9];
        up.extend([1e-9, 1e-9, 1e-9, 2e-9]);
        let cost = vec![-9.96, -3.83, -3.77, -9.52, -8.63, -1.09, -3.49, -10.98, -1.18, -284.64, -296.0, -293.99, -95.67];
        let mut is_int = vec![true; 9];
        is_int.extend([false; 4]);
        let rows = vec![
            vec![(0, 1.0), (1, 1.0), (6, 1.0), (12, 1.0)],
            vec![(3, 1.0), (1, 1.0), (5, 1.0), (12, 1.0), (9, 1.0), (11, 1.0)],
            vec![(2, 1.0), (9, 1.0), (10, 1.0)],
            vec![(0, 1.0), (12, 1.0), (9, 1.0)],
            vec![(2, -1.0), (1, 1.0), (9, 1.0)],
            vec![(6, -1.0), (0, 1.0), (12, 1.0)],
            vec![(6, -1.0), (3, 1.0), (9, 1.0)],
            vec![(1, 7.0), (7, 9.0), (5, 5.0), (3, 9.0)],
        ];
        let row_lo = vec![-inf, -inf, -inf, -inf, 0.0, 0.0, 0.0, -inf];
        let row_up = vec![1.0 - 1e-9, 1.0, 1.0 - 1e-9, 1.0 - 1e-9, 0.0, 0.0, 0.0, 5.0];
        let p = MipProblem::from_rows(vec![0.0; 13], up, cost, 0.0, 1.0, is_int, rows, row_lo, row_up);
        let mut x = vec![0.0; 13];
        for j in [4, 5, 8] {
            x[j] = 1.0;
        }
        assert!(p.is_feasible(&x, 0.0));
        let r = solve_problem(&p, MipParams { rel_gap: 1e-9, ..MipParams::default() }, true);
        assert_eq!(r.status, MipStatus::Optimal);
        let z = r.objective.unwrap();
        assert!(z <= -10.9 + 1e-6, "objective {z} (expected -10.9)");
    }

    /// 実行不能な問題を「最適」と答えた例 (analysis/objint_20261112/wrong_seed14.py、analysis/wrongans_20261112.md): 2 行目と 3 行目が
    /// `x32 - x33 >= 2` と `x32 - x33 <= 6 x13 + 7.5 x12 + 7 x23 + 6 x31` (0-1 列) で、和から `0 >= 2 + ...` になる。前処理の費用 0 の
    /// スラック列の消去 ([`crate::presolve::slackcol`]) が区間の空な行の組から x33 を消して実行可能な問題に変え、前処理なしの解き直しの
    /// 側も前処理した問題の対称性の控えで点を返していた。縮めた形 (列 3 本) と元の形 (列 34 本、17 本は行にも目的にも出ない) の両方
    #[test]
    fn infeasible_seed14_is_reported_infeasible() {
        let inf = f64::INFINITY;
        let p = MipProblem::from_rows(vec![0.0, 1.0, -4.0], vec![1.0, 58.0, inf], vec![-5.0, 0.0, 0.0], 0.0, 1.0, vec![true, false, false],
            vec![vec![(1, -1.0), (2, 1.0)], vec![(1, 1.0), (2, -1.0)]], vec![0.0, 2.0], vec![inf, inf]);
        let r = solve_problem(&p, MipParams { rel_gap: 1e-9, ..MipParams::default() }, true);
        assert_eq!(r.status, MipStatus::Infeasible, "small: objective {:?}", r.objective);
        let mut lo = vec![0.0; 34];
        lo[32] = 1.0;
        lo[33] = -4.0;
        let mut up = vec![1.0; 34];
        for (j, v) in [(0, 5.0), (6, 3.0), (8, 3.0), (9, 3.0), (10, 3.0), (18, 5.0), (21, 5.0), (27, 5.0), (32, 58.0), (33, inf)] {
            up[j] = v;
        }
        let mut cost = vec![0.0; 34];
        cost[1] = -5.0;
        let mut is_int = vec![true; 34];
        is_int[32] = false;
        is_int[33] = false;
        let mut rows = vec![
            vec![(33, 1.0), (1, -6.0), (22, -4.5), (23, -9.0), (30, -5.5), (16, -2.5)],
            vec![(33, 1.0), (32, -1.0), (13, -6.0)],
            vec![(32, 1.0), (33, -1.0), (12, -7.5), (23, -7.0), (31, -6.0)],
            vec![(11, 9.0), (4, 1.0), (0, 7.0), (21, 8.0), (7, 5.0), (20, 8.0), (28, 6.0), (31, 2.0)],
        ];
        for r in rows.iter_mut() {
            r.sort_by_key(|t| t.0);
        }
        let p = MipProblem::from_rows(lo, up, cost, 0.0, 1.0, is_int, rows, vec![-0.5, 0.0, 2.0, -inf], vec![inf, inf, inf, 19.0]);
        let r = solve_problem(&p, MipParams { rel_gap: 1e-9, ..MipParams::default() }, true);
        assert_eq!(r.status, MipStatus::Infeasible, "seed14: objective {:?}", r.objective);
    }

    /// 根の緩和が非有界で終わる問題を、区別を求められていなくても実行不能と非有界に分ける (`ENOMOTO_MIP_UNB_FEAS`、
    /// analysis/cand1113_20261112/slack240.py、analysis/wrongans_20261112.md §5)。等式 `7.5 x1 - 4.5 x2 - 7.5 x4 - 7.5 x5 - x8 = 7` は
    /// 整数解がない (左辺の x8 以外は 1.5 の倍数) が、緩和は x2・x9・x11 で目的が下に伸びる。前処理 (HNF の合同式) を通さずに解く。
    /// 右辺を 6 にすると整数解 (x1 = 2、x2 = 2) があり、x11 (行に出ない、上限なし、費用 -3) で非有界
    #[test]
    fn unbounded_root_relaxation_is_split_into_infeasible_or_unbounded() {
        let inf = f64::INFINITY;
        let build = |rhs: f64| {
            let lo = vec![0.0, -5.0, 2.0, -2.0, 0.0, 0.0, 0.0, -5.0, 0.0, -3.0, -1.0, -1.0];
            let up = vec![1.0, inf, inf, 34.0, 5.0, 1.0, 15.0, 48.0, 1.0, inf, 20.0, inf];
            let cost = vec![-5.0, 0.0, -4.0, 0.0, 0.0, 8.0, 0.0, 0.0, -5.0, -6.0, 9.0, -3.0];
            let is_int = vec![true, true, true, false, true, true, true, false, true, true, true, true];
            let rows = vec![
                vec![(6, 4.5)],
                vec![(1, -7.5), (3, 1.0), (5, -2.0), (7, -3.0)],
                vec![(4, 7.5), (5, 7.5), (6, 2.0), (10, 7.5)],
                vec![(10, -1.0)],
                vec![(1, 7.5), (2, -4.5), (4, -7.5), (5, -7.5), (8, -1.0)],
                vec![(0, -1.0), (1, 3.0), (3, 2.0), (5, 7.5), (9, 4.5)],
            ];
            MipProblem::from_rows(lo, up, cost, 0.0, 1.0, is_int, rows, vec![-5.0, -inf, -2.0, 0.0, rhs, -inf], vec![3.0, -8.0, 1.0, 7.0, rhs, 6.0])
        };
        let r = solve_problem(&build(7.0), MipParams { rel_gap: 1e-9, ..MipParams::default() }, false);
        assert_eq!(r.status, MipStatus::Infeasible);
        let r = solve_problem(&build(7.0), MipParams { rel_gap: 1e-9, ..MipParams::default() }, true);
        assert_eq!(r.status, MipStatus::Infeasible);
        let r = solve_problem(&build(6.0), MipParams { rel_gap: 1e-9, ..MipParams::default() }, false);
        assert_eq!(r.status, MipStatus::Unbounded);
        // 2 x - 2 y = 1 (x、y は上限のない整数、目的 -x): 緩和は非有界、整数解はない。木の探索だけでは終わらない形なので、
        // 実行可能性の問題では HNF の合同式 (`ENOMOTO_MIP_HNFPRE=0` でも) で実行不能を示す
        let p = MipProblem::from_rows(vec![0.0, 0.0], vec![inf, inf], vec![-1.0, 0.0], 0.0, 1.0, vec![true, true], vec![vec![(0, 2.0), (1, -2.0)]], vec![1.0], vec![1.0]);
        let t = std::time::Instant::now();
        let r = solve_problem(&p, MipParams { rel_gap: 1e-9, ..MipParams::default() }, false);
        assert_eq!(r.status, MipStatus::Infeasible);
        assert!(t.elapsed().as_secs_f64() < 30.0);
    }

    /// 幅が許容誤差以下の列の固定 ([`fix_tiny_width_columns`]): 区間の整数 (0) に固定し、整数がなければ下限。幅が許容誤差を
    /// 越える列、不等式の行の緩めの和 `Σ |a| s` が許容誤差を越える列、等式の行の破れが 1e-9 (1 + |b|) を越える列は固定しない。
    /// 不等式の行は失う余裕の分だけ緩める
    #[test]
    fn fix_tiny_width_columns_fixes_within_tolerance() {
        // 列: 0 [0, 1e-9]、1 [0.5, 0.5 + 1e-9] (整数を含まない: 下限)、2 [-1e-9, 1e-9] (0 に固定、s = 1e-9)、3 [0, 2e-6] (幅が許容誤差を越える)、
        // 4 [0, 1e-9] で係数 1e4 (緩め 1e-5)、5・6 [0, 6e-7] が同じ行 (2 本目は緩めの和が 1.2e-6 になる)、7 整数 [1, 1 - 1e-9] (交差、等式)、
        // 8 [0, 1e-8] が等式の行 (破れ 1e-8 > 1e-9)
        let inf = f64::INFINITY;
        let rows = vec![vec![(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0)], vec![(4, 1e4)], vec![(5, 1.0), (6, 1.0)], vec![(7, 1.0)], vec![(8, 1.0), (0, 1.0)]];
        let (mut rlo, mut rup) = (vec![-inf, -inf, 0.0, 1.0, 0.0], vec![2.0, 1.0, 1.0, 1.0, 0.0]);
        let mut lo = vec![0.0, 0.5, -1e-9, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let mut up = vec![1e-9, 0.5 + 1e-9, 1e-9, 2e-6, 1e-9, 6e-7, 6e-7, 1.0 - 1e-9, 1e-8];
        let is_int = vec![false, false, false, false, false, false, false, true, false];
        // サブ MIP の扱い (`submip = true`) では丸めの余裕は以前の 1e-9 (1 + |b|) のまま (外側の求解では `ENOMOTO_MIP_MEM3_TINY_MARGIN` の小さい余裕)
        let k = fix_tiny_width_columns(&rows, &mut rlo, &mut rup, &mut lo, &mut up, &is_int, true, "test");
        assert_eq!(k, 5);
        assert_eq!((lo[0], up[0]), (0.0, 0.0));
        assert_eq!((lo[1], up[1]), (0.5, 0.5));
        assert_eq!((lo[2], up[2]), (0.0, 0.0));
        assert_eq!((lo[3], up[3]), (0.0, 2e-6));
        assert_eq!((lo[4], up[4]), (0.0, 1e-9));
        assert_eq!((lo[5], up[5]), (0.0, 0.0));
        assert_eq!((lo[6], up[6]), (0.0, 6e-7));
        assert_eq!((lo[7], up[7]), (1.0, 1.0));
        assert_eq!((lo[8], up[8]), (0.0, 1e-8));
        // 不等式の行は失う余裕の分だけ緩め、等式の行はそのまま
        assert!((rup[0] - (2.0 + 3e-9 + 3e-9)).abs() < 1e-15 && rlo[0] == -inf);
        assert!((rup[2] - (1.0 + 6e-7 + 2e-9)).abs() < 1e-15 && (rlo[2] + 6e-7 + 1e-9).abs() < 1e-15);
        assert_eq!((rlo[3], rup[3]), (1.0, 1.0));
        assert_eq!((rlo[4], rup[4]), (0.0, 0.0));
    }

    /// 幅 1e-9 の連続列は MIP の前処理の入口で 0 に固定され、右辺 1 - 1e-9 の行の 0-1 列の係数に 1e-9 の大きさの値が
    /// 残らない (係数の強化が `2e-9 x + y <= 1e-9` を作らない)
    #[test]
    fn presolve_fixes_tiny_width_columns() {
        let inf = f64::INFINITY;
        // max x0 + x1 + 100 y0 + 100 y1、x0 + y0 <= 1 - 1e-9、x1 + y1 + y0 <= 1 - 1e-9、x0 + x1 <= 1.5 (x は 0-1、y は [0, 1e-9])
        let rows = vec![vec![(0, 1.0), (2, 1.0)], vec![(1, 1.0), (2, 1.0), (3, 1.0)], vec![(0, 1.0), (1, 1.0)]];
        let p = MipProblem::from_rows(vec![0.0; 4], vec![1.0, 1.0, 1e-9, 1e-9], vec![-1.0, -1.0, -100.0, -100.0], 0.0, 1.0, vec![true, true, false, false], rows, vec![-inf; 3], vec![1.0 - 1e-9, 1.0 - 1e-9, 1.5]);
        if let Some(Presolved::Reduced { prob, .. }) = presolve_mip(&p, false, false, None) {
            for j in [2, 3] {
                assert_eq!(prob.col_lo[j], prob.col_up[j], "column {j} is not fixed");
            }
            for r in &prob.rows {
                assert!(r.iter().all(|&(_, a)| a.abs() > 1e-6), "tiny coefficient left: {r:?}");
            }
        }
        let r = solve_problem(&p, MipParams { rel_gap: 1e-9, ..MipParams::default() }, true);
        assert_eq!(r.status, MipStatus::Optimal);
        assert!(r.objective.unwrap() <= -1.0 + 1e-6, "objective {:?} (expected -1)", r.objective);
    }

    /// 代入消去で生じる目的関数の定数が縮約後の offset に入ること (binkar10_1 の誤答の回帰テスト:
    /// 定数が抜けると相対ギャップの判定が誤ったスケールで働き、最適でない解で終了していた)。
    #[test]
    fn presolve_keeps_objective_constant() {
        // y + x1 + x2 = 1000 (y は連続で 1 行にしか現れないので消去される)、x1 + x2 <= 15、
        // min 1000 y + x1 + 2 x2 (y を消去すると定数 1e6 が生じる)
        let variables = vec![
            VariableData { vtype: VarType::Continuous, lb: 0.0, ub: f64::INFINITY },
            VariableData { vtype: VarType::Integer, lb: 0.0, ub: 10.0 },
            VariableData { vtype: VarType::Integer, lb: 0.0, ub: 10.0 },
        ];
        let rows = vec![vec![(0, 1.0), (1, 1.0), (2, 1.0)], vec![(1, 1.0), (2, 1.0)]];
        let p = MipProblem::from_rows(vec![0.0, 0.0, 0.0], vec![f64::INFINITY, 10.0, 10.0], vec![1000.0, 1.0, 2.0], 0.0, 1.0, vec![false, true, true], rows, vec![1000.0, f64::NEG_INFINITY], vec![1000.0, 15.0]);
        let Some(Presolved::Reduced { prob, postsolve, scaling }) = presolve_mip(&p, false, false, None) else { panic!("expected a reduced problem") };
        for (x1, x2) in [(3.0, 4.0), (10.0, 5.0), (0.0, 0.0)] {
            let mut xr: Vec<f64> = (0..prob.n).map(|j| prob.col_lo[j].max(0.0).min(prob.col_up[j])).collect();
            for (j, v) in [(1, x1), (2, x2)] {
                if prob.col_lo[j] < prob.col_up[j] {
                    xr[j] = v;
                }
            }
            let mut x = xr.clone();
            for step in postsolve.iter().rev() {
                step.apply(&mut x);
            }
            let x = crate::presolve::scaling::unscale_x(&scaling, &x);
            let (zr, z) = (prob.objective(&xr), p.objective(&x));
            assert!((zr - z).abs() <= 1e-6 * z.abs().max(1.0), "reduced objective {zr} != original {z}");
        }
    }
}

/// サブ MIP の求解の間、外側のデバッグ解 ([`solver::DEBUG_SOL`]) を退避し、終わりに戻す (診断用、`ENOMOTO_MIP_DEBUG_SOL`)
struct DebugSolScope(Option<Vec<f64>>);

impl DebugSolScope {
    fn enter() -> Self {
        Self(solver::DEBUG_SOL.with(|d| d.borrow_mut().take()))
    }
}

impl Drop for DebugSolScope {
    fn drop(&mut self) {
        if let Some(x) = self.0.take() {
            solver::DEBUG_SOL.with(|d| *d.borrow_mut() = Some(x));
        }
    }
}

/// `ENOMOTO_MIP_DUALEQ` の値 (2026-11-28 から既定 `1`: 前処理の最後に双対の議論で行を等式にし、かけ直しで縮んだときだけ使う。
/// `=0`・`off` で使わない (前の既定)、`=early` は前処理の前にかける試作)。analysis/dualeq2_20261127
pub(crate) fn dualeq() -> Option<&'static str> {
    env_def!("ENOMOTO_MIP_DUALEQ", "1")
}
