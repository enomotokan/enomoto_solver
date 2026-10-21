"""1 問を ENOMOTO で解き、返り値 (status・objective・best_bound・nodes・root_bound) を JSON で出す。
使い方: python run_one.py <mps> <秒> [スレッド数]  (環境変数はそのまま渡る)"""
import json, sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh

mps, tl = sys.argv[1], float(sys.argv[2])
threads = int(sys.argv[3]) if len(sys.argv) > 3 else 4
_, lp = bh._load_lp(Path(mps))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, tl, None, None, threads, log=True)
t = time.perf_counter() - t
print("RESULT " + json.dumps({"problem": Path(mps).stem, "status": r["status"], "objective": r["objective"], "bound": r["best_bound"],
                             "gap": r.get("mip_gap"), "nodes": r["nodes"], "root_bound": r.get("root_bound"), "time": round(t, 2)}), flush=True)
