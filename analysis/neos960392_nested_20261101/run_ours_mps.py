"""MPS (書き出したサブ MIP など) を ENOMOTO で解く。ログは ENOMOTO_MIP_LOG=1 で標準エラーに出る。

使い方: ENOMOTO_MIP_LOG=1 python run_ours_mps.py <mps> [time_limit] [threads] 2> out.log
根のサブ MIP と同じ形で解くには ENOMOTO_MIP_DEBUG_AS_SUBMIP=1 と ENOMOTO_MIP_CUTOFF=<打ち切り値> を付ける。
"""
import json
import sys
import time
from pathlib import Path

from enomoto_solver import benchmark_highs as bh

mps = Path(sys.argv[1])
tl = float(sys.argv[2]) if len(sys.argv) > 2 else 60.0
threads = int(sys.argv[3]) if len(sys.argv) > 3 else 1
_, lp = bh._load_lp(mps)
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, tl, None, None, threads)
t = time.perf_counter() - t
out = {k: r.get(k) for k in ("status", "objective", "best_bound", "nodes", "first_solution_time", "first_solution_objective")}
out["wall"] = round(t, 2)
print(json.dumps(out))
print("RESULT " + json.dumps(out), file=sys.stderr)
