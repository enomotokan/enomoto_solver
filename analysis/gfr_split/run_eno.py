"""ENOMOTO で 1 問を解く (4 スレッド・既定 60 秒)。使い方: python run_eno.py <mps> [秒]。環境変数はそのまま渡る。"""
import sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
mps = sys.argv[1]
T = float(sys.argv[2]) if len(sys.argv) > 2 else 60.0
_, lp = bh._load_lp(Path(mps))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, T, None, None, 4, log=True)
print("RESULT", r["status"], r["objective"], r["best_bound"], r["nodes"], round(time.perf_counter() - t, 1), r.get("root_bound"), flush=True)
