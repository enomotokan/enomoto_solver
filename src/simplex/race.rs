//! 傾き・切片双対二段解法と内点法 + クロスオーバーの同時実行 (`RootSolver::Auto`、前処理後の行数が
//! `RACE_MIN_ROWS` 以上の問題)。
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

    {
        let (std, cancel, tx) = (std.clone(), cancel.clone(), tx.clone());
        std::thread::spawn(move || {
            let r = pool_s.install(|| crate::cancel::with_token(Some(cancel), || solve_std_form_decomposed(&std, &opts)));
            let _ = tx.send((Engine::Simplex, Some(r)));
        });
    }
    {
        let (std, cancel, tx) = (std.clone(), cancel.clone(), tx.clone());
        std::thread::spawn(move || {
            let r = pool_i.install(|| crate::cancel::with_token(Some(cancel), || crossover::solve_ipm_crossover(&std)));
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
