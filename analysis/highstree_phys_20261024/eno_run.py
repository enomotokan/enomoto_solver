"""ENOMOTO を 1 回解く: python eno_run.py <mps> <秒> [スレッド]。最後の行に JSON。"""
import json, sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh

mps, T = sys.argv[1], float(sys.argv[2])
th = int(sys.argv[3]) if len(sys.argv) > 3 else 4
_, lp = bh._load_lp(Path(mps))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, T, None, None, th, log=True)
t = time.perf_counter() - t
print(json.dumps({k: r.get(k) for k in ["status", "objective", "best_bound", "nodes", "root_bound",
                                        "first_solution_time", "first_solution_objective"]} | {"time": t}), flush=True)
