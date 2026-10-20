import sys, json, time, os
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
import enomoto_solver
mps, T, th = sys.argv[1], float(sys.argv[2]), int(sys.argv[3])
print("MODULE", enomoto_solver.__file__, file=sys.stderr, flush=True)
_, lp = bh._load_lp(Path(mps)); m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, T, None, None, th, log=True)
t = time.perf_counter() - t
print("RESULT " + json.dumps({k: r.get(k) for k in ("status","objective","best_bound","nodes","root_bound","first_solution_time","first_solution_objective")} | {"time": t}), flush=True)
