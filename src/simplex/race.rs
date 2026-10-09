//! 傾き・切片双対二段解法と内点法 + クロスオーバーの同時実行 (`RootSolver::Auto`、前処理後の行数が
//! `RACE_MIN_ROWS` (1000) 以上の問題)。
//!
//! 前処理後の標準形を 2 つのスレッドで別々に解き、先に結論 (最適、実行不能、非有界など `NotSolved` 以外)
//! を出した側を採用する。
//!
//! - **スレッドの配分**: 二段解法は反復の本体が逐次なので、`RACE_SIMPLEX_THREADS` (既定 1) スレッドの専用
//!   rayon プール (連結成分の並列求解に使う) で動かし、内点法 + クロスオーバーは残りのスレッドの専用プールで
//!   動かす。プールを分けるので、内点法の Cholesky が全スレッドを埋めて二段解法の並列区間を待たせる
//!   (またはその逆) ことはない。
//! - **打ち切り**: 勝った側が [`crate::cancel`] のトークンを立て、負けた側は反復ループの確認点で結論なしに
//!   戻る。勝った結果はすぐ返し、負けた側のスレッドは待たない (内点法の 1 反復が長い問題で、打ち切りの
//!   確認点までの時間を待たないため)。負けた側はトークンを見て間もなく止まり、そのプールも破棄される。
//! - **解き直しなし**: 内点法が収束しなくても二段解法で解き直さない (二段解法が既に走っている)。
//! - **成分ごと**: 独立な成分への分割は前処理の直後に 1 回だけ行い (`simplex::solve_presolved`)、同時実行は
//!   成分 (組) ごとに呼ばれる。ここでは分割し直さない。
//! - **状態**: 1 回の求解ごとの状態 (LU のピボット閾値、解き直しの印、費用摂動の切り替え) はスレッドローカル
//!   なので混ざらない。節目の時刻の記録 (`phase_timing`) は両方の節目が混ざって入る (名前で区別できる)。
//!   勝った側を `race_simplex_won` / `race_ipm_crossover_won` で記録する。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

use super::{crossover, solve_simplex_block, SimplexResult, StdForm};
use crate::params::simplex::RACE_SIMPLEX_THREADS;
use crate::types::Status;

/// どちらのエンジンの結果か。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Engine {
    Simplex,
    IpmCrossover,
}

/// `std` を 2 つのエンジンで同時に解き、先に結論を出した側の結果を返す (両方とも結論なしなら `NotSolved`)。
pub(super) fn solve_race(std: Arc<StdForm>, opts: crate::types::LpOptions) -> SimplexResult {
    // 小さな問題 (非零の数が `ENOMOTO_T_RACE_SMALL_NNZ` 未満、既定は `XO_SERIAL_NNZ`) は、プールを作り直さず
    // 二段解法を呼び出し元のスレッドで、内点法を使い回しの 1 スレッドのプールで解く ([`solve_race_small`])。
    let small_nnz = tunable!("ENOMOTO_T_RACE_SMALL_NNZ", crate::params::simplex::XO_SERIAL_NNZ, usize);
    if std.cols.nnz() < small_nnz {
        return solve_race_small(std, opts);
    }
    let total = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).max(2);
    let s_threads = tunable!("ENOMOTO_T_RACE_SIMPLEX_THREADS", RACE_SIMPLEX_THREADS, usize).clamp(1, total - 1);
    // 試験用 (`ENOMOTO_T_RACE_IPM_THREADS=k`、0 = 残り全部): 内点法側のスレッド数。論理 CPU が多い計算機で、内点法が
    // 残りのスレッドを埋めると、1 スレッドの二段解法がメモリ帯域や同じ物理コア (SMT) の取り合いで遅くなる
    // (pds-100: 16 論理 CPU のノートで二段解法だけなら 107 秒、auto では 304 秒)。
    let i_req = tunable!("ENOMOTO_T_RACE_IPM_THREADS", 0usize, usize);
    let i_threads = if i_req == 0 { total - s_threads } else { i_req.min(total - s_threads) };
    let build = |k: usize, name: &'static str| {
        rayon::ThreadPoolBuilder::new().num_threads(k).thread_name(move |i| format!("enomoto-{name}-{i}")).build()
    };
    let (pool_s, pool_i) = match (build(s_threads, "simplex"), build(i_threads, "ipm")) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return solve_simplex_block(&std, &opts),
    };
    let debug = env_str!("ENOMOTO_DEBUG_RACE").is_some();
    let t0 = std::time::Instant::now();
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel::<(Engine, Option<SimplexResult>)>();
    // 二段解法 (段階 B) が書き、内点法が読む元の問題の最適値の下界。
    let bound = crate::cancel::SharedBound::new(std.n_total, std.n_rows);

    {
        let (std, cancel, tx, bound) = (std.clone(), cancel.clone(), tx.clone(), bound.clone());
        std::thread::spawn(move || {
            let r = pool_s.install(|| {
                crate::cancel::with_token(Some(cancel), || crate::cancel::with_bound(Some(bound), || solve_simplex_block(&std, &opts)))
            });
            let _ = tx.send((Engine::Simplex, Some(r)));
        });
    }
    {
        let (std, cancel, tx, bound) = (std.clone(), cancel.clone(), tx.clone(), bound.clone());
        std::thread::spawn(move || {
            let r = pool_i.install(|| {
                crate::cancel::with_token(Some(cancel), || crate::cancel::with_bound(Some(bound), || crossover::solve_ipm_crossover(&std)))
            });
            let _ = tx.send((Engine::IpmCrossover, r));
        });
    }
    drop(tx);

    let mut finished = 0;
    while let Ok((engine, r)) = rx.recv() {
        finished += 1;
        if debug {
            eprintln!("RACE {engine:?} finished at {:.3}s with {:?}", t0.elapsed().as_secs_f64(), r.as_ref().map(|r| r.status.clone()));
        }
        if let Some(r) = r {
            if r.status != Status::NotSolved && !cancel.swap(true, Ordering::SeqCst) {
                crate::phase_timing::mark(match engine {
                    Engine::Simplex => "race_simplex_won",
                    Engine::IpmCrossover => "race_ipm_crossover_won",
                });
                return r;
            }
        }
        if finished == 2 {
            break;
        }
    }
    SimplexResult { status: Status::NotSolved, x: None }
}

/// 独立な組 (成分) が 2 つ以上あるときの同時実行: スレッドの配分は [`solve_race`] と同じ (二段解法
/// `RACE_SIMPLEX_THREADS`、内点法 + クロスオーバーは残り) のまま、2 つの側が組の一覧を手分けして解き、組ごとに先着を採る。
/// 内点法側は大きな組から並列に、二段解法側は小さな組から順に解く。組ごとに打ち切りのトークンと共有の下界を持ち、
/// どちらかが先に結論を出した組はもう一方が飛ばす (解いている途中なら確認点で止まる)。組を 1 つずつ同時実行すると、
/// 小さな組では内点法の並列が効かずに待ちが積み上がる (fome13 の 8 成分: 18 → 35 秒)。組ごとに先着を採るので、内点法が
/// 外れた組だけを二段解法が拾える (全体を 1 つの同時実行にすると、1 組の外れで二段解法の全体を待つ: fome13 で 83 秒)。
/// 戻り値は `subs` と同じ順。どちらの側も結論を出せなかった組は `NotSolved`。
pub(super) fn solve_race_groups(subs: Vec<Arc<StdForm>>, opts: crate::types::LpOptions) -> Vec<SimplexResult> {
    let k = subs.len();
    let total = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).max(2);
    let s_threads = tunable!("ENOMOTO_T_RACE_SIMPLEX_THREADS", RACE_SIMPLEX_THREADS, usize).clamp(1, total - 1);
    let i_req = tunable!("ENOMOTO_T_RACE_IPM_THREADS", 0usize, usize);
    let i_threads = if i_req == 0 { total - s_threads } else { i_req.min(total - s_threads) };
    let build = |n: usize, name: &'static str| {
        rayon::ThreadPoolBuilder::new().num_threads(n).thread_name(move |i| format!("enomoto-{name}-{i}")).build()
    };
    let (pool_s, pool_i) = match (build(s_threads, "simplex"), build(i_threads, "ipm")) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return subs.iter().map(|s| solve_simplex_block(s, &opts)).collect(),
    };
    let tokens: Arc<Vec<Arc<AtomicBool>>> = Arc::new((0..k).map(|_| Arc::new(AtomicBool::new(false))).collect());
    let bounds: Arc<Vec<crate::cancel::SharedBound>> = Arc::new(subs.iter().map(|s| crate::cancel::SharedBound::new(s.n_total, s.n_rows)).collect());
    // 外側の打ち切り (この呼び出し自体が別の同時実行の中にあるとき) を各組のトークンに伝える。
    let outer = crate::cancel::current();
    // 大きい順 (行列の非零の数)。内点法側はこの順、二段解法側は逆順に解く。
    let mut order: Vec<usize> = (0..k).collect();
    order.sort_by_key(|&g| std::cmp::Reverse(subs[g].cols.nnz()));
    let order = Arc::new(order);
    let subs = Arc::new(subs);
    let (tx, rx) = mpsc::channel::<(usize, Engine, SimplexResult)>();
    {
        let (subs, tokens, bounds, order, tx) = (subs.clone(), tokens.clone(), bounds.clone(), order.clone(), tx.clone());
        std::thread::spawn(move || {
            pool_s.install(|| {
                for &g in order.iter().rev() {
                    if tokens[g].load(Ordering::SeqCst) {
                        continue;
                    }
                    let r = crate::cancel::with_token(Some(tokens[g].clone()), || crate::cancel::with_bound(Some(bounds[g].clone()), || solve_simplex_block(&subs[g], &opts)));
                    if tx.send((g, Engine::Simplex, r)).is_err() {
                        break;
                    }
                }
            });
        });
    }
    {
        let (subs, tokens, bounds, order, tx) = (subs.clone(), tokens.clone(), bounds.clone(), order.clone(), tx.clone());
        std::thread::spawn(move || {
            use rayon::prelude::*;
            pool_i.install(|| {
                order.par_iter().with_max_len(1).for_each(|&g| {
                    if tokens[g].load(Ordering::SeqCst) {
                        return;
                    }
                    let r = crate::cancel::with_token(Some(tokens[g].clone()), || crate::cancel::with_bound(Some(bounds[g].clone()), || crossover::solve_ipm_crossover(&subs[g])));
                    let _ = tx.send((g, Engine::IpmCrossover, r.unwrap_or(SimplexResult { status: Status::NotSolved, x: None })));
                });
            });
        });
    }
    drop(tx);
    let mut results: Vec<Option<SimplexResult>> = (0..k).map(|_| None).collect();
    // 組ごとに、結論なしで戻った側の数 (2 になったらその組は `NotSolved`)。
    let mut gave_up = vec![0u8; k];
    let mut open = k;
    while open > 0 {
        let Ok((g, engine, r)) = rx.recv_timeout(std::time::Duration::from_millis(50)) else {
            if outer.as_ref().is_some_and(|t| t.load(Ordering::Relaxed)) {
                for t in tokens.iter() {
                    t.store(true, Ordering::SeqCst);
                }
                break;
            }
            continue;
        };
        if results[g].is_some() {
            continue;
        }
        if r.status != Status::NotSolved && !tokens[g].swap(true, Ordering::SeqCst) {
            crate::phase_timing::mark(match engine {
                Engine::Simplex => "race_simplex_won",
                Engine::IpmCrossover => "race_ipm_crossover_won",
            });
            results[g] = Some(r);
            open -= 1;
        } else if r.status == Status::NotSolved {
            gave_up[g] += 1;
            if gave_up[g] == 2 {
                results[g] = Some(r);
                open -= 1;
            }
        }
    }
    // 結論の出た組は、負けた側が確認点で止まるのを待たずに返す。
    for t in tokens.iter() {
        t.store(true, Ordering::SeqCst);
    }
    results.into_iter().map(|r| r.unwrap_or(SimplexResult { status: Status::NotSolved, x: None })).collect()
}

/// 小さな問題の同時実行: 二段解法は呼び出し元のスレッドで、内点法 + クロスオーバーは使い回しの 1 スレッドの
/// プールで解く (スレッドの生成・起床の手間を省く。仮想マシンでは 1 回に数ミリ秒かかることがある)。
/// 先に結論を出した側がトークンを立て、もう一方は確認点で止まる。二段解法が先に戻ったら (結論なしで止められた
/// 場合も) 内点法の結果を待つ。
fn solve_race_small(std: Arc<StdForm>, opts: crate::types::LpOptions) -> SimplexResult {
    static POOL: std::sync::OnceLock<Option<rayon::ThreadPool>> = std::sync::OnceLock::new();
    let Some(pool) = POOL.get_or_init(|| rayon::ThreadPoolBuilder::new().num_threads(1).thread_name(|_| "enomoto-ipm-small".into()).build().ok())
    else {
        return solve_simplex_block(&std, &opts);
    };
    let debug = env_str!("ENOMOTO_DEBUG_RACE").is_some();
    let t0 = std::time::Instant::now();
    let cancel = Arc::new(AtomicBool::new(false));
    let bound = crate::cancel::SharedBound::new(std.n_total, std.n_rows);
    let (tx, rx) = mpsc::channel::<Option<SimplexResult>>();
    {
        let (std, cancel, bound) = (std.clone(), cancel.clone(), bound.clone());
        pool.spawn(move || {
            let r = crate::cancel::with_token(Some(cancel.clone()), || crate::cancel::with_bound(Some(bound), || crossover::solve_ipm_crossover(&std)));
            // 結論が出たら先に立てて二段解法を止める (二段解法が先なら立っている)。
            let won = r.as_ref().is_some_and(|r| r.status != Status::NotSolved) && !cancel.swap(true, Ordering::SeqCst);
            let _ = tx.send(if won { r } else { None });
        });
    }
    let rs = crate::cancel::with_token(Some(cancel.clone()), || crate::cancel::with_bound(Some(bound), || solve_simplex_block(&std, &opts)));
    if rs.status != Status::NotSolved && !cancel.swap(true, Ordering::SeqCst) {
        if debug {
            eprintln!("RACE(small) simplex won at {:.3}s", t0.elapsed().as_secs_f64());
        }
        crate::phase_timing::mark("race_simplex_won");
        return rs;
    }
    // 内点法が先に結論を出した (または二段解法が結論なしで戻った): 内点法の結果を待つ。
    match rx.recv() {
        Ok(Some(r)) => {
            if debug {
                eprintln!("RACE(small) ipm_crossover won at {:.3}s", t0.elapsed().as_secs_f64());
            }
            crate::phase_timing::mark("race_ipm_crossover_won");
            r
        }
        _ => rs,
    }
}
