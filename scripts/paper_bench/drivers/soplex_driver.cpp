// 論文用ベンチマークの SoPlex ドライバ (scripts/paper_bench/run_paper_bench.py から呼ぶ)。
//
// 使い方: soplex_driver <問題.mps> <制限秒>
//         soplex_driver --version
//
// MPS を読み込んでから (読み込み時間は計時しない)、既定のパラメータ (時間制限と実時間タイマー
// だけ設定) で optimize() し、その経過時間 (steady_clock、実時間) と状態を JSON 1 行で
// 標準出力に書く。
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "soplex.h"

#ifndef SOPLEX_VERSION_STR
#define SOPLEX_VERSION_STR "unknown"
#endif

using namespace soplex;
using Solver = SPxSolverBase<Real>;

static const char* status_name(Solver::Status s) {
    switch (s) {
        case Solver::OPTIMAL: return "optimal";
        case Solver::INFEASIBLE: return "infeasible";
        case Solver::UNBOUNDED: return "unbounded";
        case Solver::INForUNBD: return "infeasible_or_unbounded";
        case Solver::ABORT_TIME: return "limit";
        case Solver::ABORT_ITER: return "limit";
        default: return "other";
    }
}

int main(int argc, char** argv) {
    if (argc >= 2 && std::strcmp(argv[1], "--version") == 0) {
        std::printf("%s\n", SOPLEX_VERSION_STR);
        return 0;
    }
    if (argc < 3) {
        std::fprintf(stderr, "usage: %s problem.mps time_limit_seconds\n", argv[0]);
        return 2;
    }
    using clock = std::chrono::steady_clock;
    const double limit = std::atof(argv[2]);

    SoPlex solver;
    solver.setIntParam(SoPlex::VERBOSITY, SoPlex::VERBOSITY_ERROR);
    solver.setIntParam(SoPlex::TIMER, SoPlex::TIMER_WALLCLOCK);
    auto t0 = clock::now();
    const bool ok = solver.readFile(argv[1]);
    const double read_time = std::chrono::duration<double>(clock::now() - t0).count();
    if (!ok) {
        std::printf("{\"status\": \"read_error\"}\n");
        return 0;
    }
    solver.setRealParam(SoPlex::TIMELIMIT, limit);
    // 読み込み完了の合図 (親はここから制限時間を数える)。
    std::printf("{\"built\": true, \"read_time\": %.9g}\n", read_time);
    std::fflush(stdout);

    auto t1 = clock::now();
    const Solver::Status st = solver.optimize();
    const double solve_time = std::chrono::duration<double>(clock::now() - t1).count();

    const double obj = st == Solver::OPTIMAL ? double(solver.objValueReal()) : 0.0;
    std::printf(
        "{\"time\": %.9g, \"read_time\": %.9g, \"status\": \"%s\", \"raw_status\": %d, "
        "\"obj\": %.17g, \"iters\": %d}\n",
        solve_time, read_time, status_name(st), int(st), obj, solver.numIterations());
    return 0;
}
