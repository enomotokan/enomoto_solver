# ENOMOTO を 4 スレッドで解く: python run_eno.py <mps> <T>
import sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
mps, T = sys.argv[1], float(sys.argv[2])
_, lp = bh._load_lp(Path(mps)); m, _, _ = bh._build_our_model(lp)
t = time.time(); r = m.solve(None, False, T, None, None, 4, log=True)
print("RESULT", Path(mps).name, f"{time.time()-t:.2f}", r if not hasattr(r, '__dict__') else r.__dict__, flush=True)
