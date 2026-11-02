"""ENOMOTO で 1 問を解く (2026-11-02、analysis/ns1116954_20261101/run_eno.py と同じ呼び方: 4 スレッド・60 秒)。
使い方: python run_eno.py <mps> <問題名> <形> <回>  (環境変数はそのまま渡す。結果を 1 行の JSON で標準出力に出す)"""
import json, os, sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, 60.0, None, None, 4)
t = time.perf_counter() - t
env = {k: v for k, v in os.environ.items() if k.startswith("ENOMOTO_") and k != "ENOMOTO_MIP_LOG"}
print(json.dumps({"problem": sys.argv[2], "mode": sys.argv[3], "run": int(sys.argv[4]), "status": r["status"], "objective": r["objective"],
                  "bound": r["best_bound"], "nodes": r["nodes"], "time": round(t, 2), "env": env}), flush=True)
