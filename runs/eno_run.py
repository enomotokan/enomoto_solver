import sys, time, os
from pathlib import Path
import enomoto_solver
from enomoto_solver import benchmark_highs as bh
print("MODULE", enomoto_solver.__file__, flush=True)
p, T = sys.argv[1], float(sys.argv[2])
_, lp = bh._load_lp(Path(p)); m, _, _ = bh._build_our_model(lp)
t = time.time()
r = m.solve(None, False, T, None, None, int(os.environ.get("NTHR","4")), log=True)
print("RESULT", time.time() - t, r, flush=True)
