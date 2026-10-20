"""ENOMOTO を 1 回解く: python run_eno.py <mps> <time_limit> [threads]  (ENOMOTO_MIP_THREADS は呼ぶ側で)"""
import sys, time, json
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
mps, T = sys.argv[1], float(sys.argv[2])
th = int(sys.argv[3]) if len(sys.argv) > 3 else 4
t0 = time.perf_counter()
_, lp = bh._load_lp(Path(mps))
m, _, _ = bh._build_our_model(lp)
print(f"load {time.perf_counter()-t0:.1f}s", flush=True)
t = time.perf_counter()
r = m.solve(None, False, T, None, None, th, log=True)
t = time.perf_counter() - t
keys = ["status","objective","best_bound","nodes","root_bound","first_solution_time","first_solution_objective"]
d = {k: r.get(k) for k in keys}; d["time"] = t
print("RESULT " + json.dumps(d), flush=True)
