import sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
mps = Path(__import__("os").environ.get("MPS", "/home/user/enomoto_solver/.miplib_cache/neos-911970.mps"))
T = float(sys.argv[1]) if len(sys.argv) > 1 else 60.0
_, lp = bh._load_lp(mps); m, _, _ = bh._build_our_model(lp)
t = time.time()
r = m.solve(None, False, T, None, None, int(__import__("os").environ.get("NTH","4")), log=True)
print("RESULT", time.time() - t, r if not hasattr(r, '__dict__') else r.__dict__, flush=True)
