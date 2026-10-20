import sys, os, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
import enomoto_solver; print(enomoto_solver.__file__, flush=True)
T=float(sys.argv[2]) if len(sys.argv)>2 else 60
_, lp = bh._load_lp(Path(sys.argv[1])); m, _, _ = bh._build_our_model(lp)
t=time.time(); r = m.solve(None, False, T, None, None, int(os.environ.get("NT","4")), log=True)
print("RESULT", round(time.time()-t,2), {k:v for k,v in r.items() if k!="x"}, flush=True)
