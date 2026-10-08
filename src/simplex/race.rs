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
//! - **状態**: 1 回の求解ごとの状態 (LU のピボット閾値、解き直しの印、費用摂動の切り替え) はスレッドローカル
//!   なので混ざらない。節目の時刻の記録 (`phase_timing`) は両方の節目が混ざって入る (名前で区別できる)。
//!   勝った側を `race_simplex_won` / `race_ipm_crossover_won` で記録する。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

use super::{crossover, solve_std_form_decomposed, SimplexResult, StdForm};
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
    let i_threads = total - s_threads;
    let build = |k: usize, name: &'static str| {
        rayon::ThreadPoolBuilder::new().num_threads(k).thread_name(move |i| format!("enomoto-{name}-{i}")).build()
    };
    let (pool_s, pool_i) = match (build(s_threads, "simplex"), build(i_threads, "ipm")) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return solve_std_form_decomposed(&std, &opts),
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
                crate::cancel::with_token(Some(cancel), || crate::cancel::with_bound(Some(bound), || solve_std_form_decomposed(&std, &opts)))
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

/// 小さな問題の同時実行: 二段解法は呼び出し元のスレッドで、内点法 + クロスオーバーは使い回しの 1 スレッドの
/// プールで解く (スレッドの生成・起床の手間を省く。仮想マシンでは 1 回に数ミリ秒かかることがある)。
/// 先に結論を出した側がトークンを立て、もう一方は確認点で止まる。二段解法が先に戻ったら (結論なしで止められた
/// 場合も) 内点法の結果を待つ。
fn solve_race_small(std: Arc<StdForm>, opts: crate::types::LpOptions) -> SimplexResult {
    static POOL: std::sync::OnceLock<Option<rayon::ThreadPool>> = std::sync::OnceLock::new();
    let Some(pool) = POOL.get_or_init(|| rayon::ThreadPoolBuilder::new().num_threads(1).thread_name(|_| "enomoto-ipm-small".into()).build().ok())
    else {
        return solve_std_form_decomposed(&std, &opts);
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
    let rs = crate::cancel::with_token(Some(cancel.clone()), || crate::cancel::with_bound(Some(bound), || solve_std_form_decomposed(&std, &opts)));
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
