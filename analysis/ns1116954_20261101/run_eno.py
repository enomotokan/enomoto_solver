"""ENOMOTO で 1 問を解く (2026-11-01)。ENOMOTO_MIP_LOG=1 などは環境変数で渡す。ログは標準エラーに出る。
使い方: python run_eno.py <mps>  (4 スレッド・60 秒、ベンチマークの worker と同じ呼び方)"""
import json, os, sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, 60.0, None, None, 4)
t = time.perf_counter() - t
env = {k: v for k, v in os.environ.items() if k.startswith("ENOMOTO_") and k != "ENOMOTO_MIP_LOG"}
print(json.dumps({"status": r["status"], "objective": r["objective"], "bound": r["best_bound"], "nodes": r["nodes"],
                  "time": round(t, 2), "first_solution_time": r.get("first_solution_time"), "env": env}), flush=True)
