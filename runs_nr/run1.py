"""1 問を 4 スレッドで解き、結果を JSON 1 行で標準出力に出す (標準エラーには RESTDIAG などの診断)。
使い方: python run1.py <mps> <秒>"""
import json, sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh

mps, tl = sys.argv[1], float(sys.argv[2])
_, lp = bh._load_lp(Path(mps))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
import os
th = int(os.environ.get("NR_THREADS", "4"))
r = m.solve(None, False, tl, None, None, th)
t = time.perf_counter() - t
print(json.dumps({"status": r["status"], "objective": r["objective"], "bound": r["best_bound"], "nodes": r["nodes"], "time": round(t, 2), "maxrss_mb": round(__import__("resource").getrusage(__import__("resource").RUSAGE_SELF).ru_maxrss / 1024)}))
