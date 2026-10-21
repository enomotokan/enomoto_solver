import sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
mps, threads, tl = sys.argv[1], int(sys.argv[2]), float(sys.argv[3])
_, lp = bh._load_lp(Path(mps))
m, _, _ = bh._build_our_model(lp)
t = time.time()
r = m.solve(None, False, tl, None, None, threads, log=True)
st = r["status"]; obj = r["objective"]; bd = r["best_bound"]
print(f"RESULT status={st} obj={obj} bound={bd} time={time.time()-t:.2f}", flush=True)
