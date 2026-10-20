# ENOMOTO を 1 問だけ動かす (python eno_run.py <mps> <秒>)。スレッド数は環境変数 NTHR (既定 4)。結果は x を除いて出す
import sys, time, os
from pathlib import Path
import enomoto_solver
from enomoto_solver import benchmark_highs as bh
print("MODULE", enomoto_solver.__file__, flush=True)
p, T = sys.argv[1], float(sys.argv[2])
_, lp = bh._load_lp(Path(p)); m, _, _ = bh._build_our_model(lp)
t = time.time()
r = m.solve(None, False, T, None, None, int(os.environ.get("NTHR", "4")), log=True)
r = dict(r); r.pop("x", None)
print("RESULT", round(time.time() - t, 2), r, flush=True)
