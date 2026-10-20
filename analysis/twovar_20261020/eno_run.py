import os, sys, time
os.environ.setdefault("ENOMOTO_MIP_THREADS","4")
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
print(__import__('enomoto_solver').__file__)
_, lp = bh._load_lp(Path(sys.argv[1])); m,_,_ = bh._build_our_model(lp)
t=time.time(); r=m.solve(None, False, float(sys.argv[2]), None, None, 4, log=True)
r=dict(r); r.pop("x",None)
print("RESULT", r, time.time()-t)
