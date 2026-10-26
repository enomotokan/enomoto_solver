"""ENOMOTO を 1 問だけ 4 スレッドで解く。使い方: python run_ours.py <mps> [time]  (環境変数はそのまま渡る)"""
import sys, time, json
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
mps = Path(sys.argv[1]); T = float(sys.argv[2]) if len(sys.argv) > 2 else 60.0
_, lp = bh._load_lp(mps)
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, T, None, None, 4, log=True)
t = time.perf_counter() - t
print("RESULT", json.dumps({k: r.get(k) for k in ["status", "objective", "best_bound", "nodes", "first_solution_time", "first_solution_objective"]} | {"time": t}), flush=True)
