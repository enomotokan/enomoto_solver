import sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
mps, T = sys.argv[1], float(sys.argv[2])
_, lp = bh._load_lp(Path(mps)); m, _, _ = bh._build_our_model(lp)
import os
t=time.time(); r = m.solve(None, False, T, None, None, int(os.environ.get("THR","4")), log=True)
print("RESULT", time.time()-t, r if not hasattr(r,'__dict__') else r.__dict__, flush=True)
