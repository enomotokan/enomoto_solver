import sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
prob, T = sys.argv[1], float(sys.argv[2])
_, lp = bh._load_lp(Path(f"/home/user/enomoto_solver/.miplib_cache/{prob}.mps")); m, _, _ = bh._build_our_model(lp)
t=time.time()
r = m.solve(None, False, T, None, None, 4, log=True)
print("RESULT", prob, time.time()-t, r if not hasattr(r,'__dict__') else r.__dict__, flush=True)
