"""run_long.py の kind=ourslog 用: --worker-ours と同じ求解に log=True を付け、利用者向けの経過 (全スレッドの済んだノード・開いたノード
(共有の待ち行列を含む)) を 5 秒ごとに標準出力に出す (2026-11-03)。使い方: python diag_worker.py MPS 時間の上限"""
import json, sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, float(sys.argv[2]), None, None, 4, True)
t = time.perf_counter() - t
print(json.dumps({"status": r["status"], "objective": r["objective"], "bound": r["best_bound"], "nodes": r["nodes"], "time": t}), flush=True)
