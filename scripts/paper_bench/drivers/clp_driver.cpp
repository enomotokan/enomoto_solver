// 論文用ベンチマークの CLP ドライバ (scripts/paper_bench/run_paper_bench.py から呼ぶ)。
//
// 使い方: clp_driver <問題.mps> <制限秒>
//         clp_driver --version
//
// MPS を読み込んでから (読み込み時間は計時しない)、既定の ClpSolve (前処理あり、解法は自動選択)
// で initialSolve() し、その経過時間 (steady_clock、実時間) と状態を JSON 1 行で標準出力に書く。
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "ClpSimplex.hpp"
#include "ClpSolve.hpp"

#ifndef CLP_VERSION_STR
#define CLP_VERSION_STR "unknown"
#endif

static const char* status_name(int s) {
    switch (s) {
        case 0: return "optimal";
        case 1: return "infeasible";
        case 2: return "unbounded";
        case 3: return "limit";
        case 4: return "error";
        case 5: return "stopped";
        default: return "other";
    }
}

int main(int argc, char** argv) {
    if (argc >= 2 && std::strcmp(argv[1], "--version") == 0) {
        std::printf("%s\n", CLP_VERSION_STR);
        return 0;
    }
    if (argc < 3) {
        std::fprintf(stderr, "usage: %s problem.mps time_limit_seconds\n", argv[0]);
        return 2;
    }
    using clock = std::chrono::steady_clock;
    const double limit = std::atof(argv[2]);

    ClpSimplex model;
    model.setLogLevel(0);
    auto t0 = clock::now();
    const int errors = model.readMps(argv[1], true, false);
    const double read_time = std::chrono::duration<double>(clock::now() - t0).count();
    if (errors != 0) {
        std::printf("{\"status\": \"read_error\", \"detail\": %d}\n", errors);
        return 0;
    }
    model.setMaximumSeconds(limit);
    model.setMaximumWallSeconds(limit);
    // 読み込み完了の合図 (親はここから制限時間を数える)。
    std::printf("{\"built\": true, \"read_time\": %.9g}\n", read_time);
    std::fflush(stdout);

    ClpSolve options;  // 既定: 前処理あり、解法は自動選択 (clp のコマンドライン -solve と同じ)
    auto t1 = clock::now();
    model.initialSolve(options);
    const double solve_time = std::chrono::duration<double>(clock::now() - t1).count();

    const int st = model.status();
    std::printf(
        "{\"time\": %.9g, \"read_time\": %.9g, \"status\": \"%s\", \"raw_status\": %d, "
        "\"secondary_status\": %d, \"obj\": %.17g, \"iters\": %d}\n",
        solve_time, read_time, status_name(st), st, model.secondaryStatus(),
        model.objectiveValue(), model.numberIterations());
    return 0;
}
