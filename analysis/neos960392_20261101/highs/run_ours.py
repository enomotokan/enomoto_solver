"""neos-960392 を ENOMOTO (今の既定) で解く (4 スレッド・60 秒)。ログは ENOMOTO_MIP_LOG=1 を付けて標準エラーに出る。

使い方: ENOMOTO_MIP_LOG=1 python run_ours.py 2> out.log
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
from enomoto_solver import benchmark_highs as bh  # noqa: E402

mps = rb.fetch("neos-960392", Path(".miplib_cache"))
_, lp = bh._load_lp(mps)
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, 60.0, None, None, 4)
t = time.perf_counter() - t
out = {k: r.get(k) for k in ("status", "objective", "best_bound", "nodes", "root_bound",
                              "first_solution_time", "first_solution_objective")}
out["wall"] = round(t, 2)
print(json.dumps(out))
print("RESULT " + json.dumps(out), file=sys.stderr)
