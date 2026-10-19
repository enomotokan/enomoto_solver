import sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
name, T = sys.argv[1], float(sys.argv[2])
mps = f"/home/user/enomoto_solver/.miplib_cache/{name}.mps"
_, lp = bh._load_lp(Path(mps)); m, _, _ = bh._build_our_model(lp)
t=time.time()
r = m.solve(None, False, T, None, None, 4, log=True)
d = r if not hasattr(r,'__dict__') else dict(r.__dict__)
x = d.pop('x', None) if isinstance(d, dict) else None
print("RESULT", name, time.time()-t, d, flush=True)
import os
out = os.environ.get("SOLOUT")
if out and x is not None:
    open(out,"w").write("\n".join(repr(float(v)) for v in x)+"\n")
