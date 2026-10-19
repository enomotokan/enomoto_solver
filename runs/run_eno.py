import sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
mps=sys.argv[1]; T=float(sys.argv[2]); th=int(sys.argv[3]) if len(sys.argv)>3 else 1
_, lp = bh._load_lp(Path(mps)); m,_,_ = bh._build_our_model(lp)
t=time.time(); r=m.solve(None, False, T, None, None, th, log=True)
print("RESULT", time.time()-t, r)
