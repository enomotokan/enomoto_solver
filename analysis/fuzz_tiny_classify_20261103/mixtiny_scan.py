"""fuzz_mixtiny の seed を順に解き、panic・食い違いを seed ごとに出す (panic でも止めない)。
使い方: python analysis/fuzz_tiny_classify_20261103/mixtiny_scan.py 問題数 [開始seed]"""
import sys
FM = "benchmarks/tinywidth_20261022/fuzz_mixtiny.py"
src = open(FM).read().rsplit("N=int(sys.argv[1])", 1)[0]
ns = {"__file__": FM}; exec(compile(src, FM, "exec"), ns)
N = int(sys.argv[1]); st = int(sys.argv[2]) if len(sys.argv) > 2 else 0
for seed in range(st, st + N):
    lp = ns["gen"](seed); hs, ho = ns["highs"](lp)
    try:
        os_, oo, x = ns["ours"](lp)
    except BaseException as e:
        print("seed", seed, "PANIC", type(e).__name__, str(e)[:80], "highs", hs, ho, flush=True); continue
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible"}.get(hs, hs)
    if not ((hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))):
        print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
