"""58 問 (analysis/cbcimpl_20261029/probs.py) を今の既定で 1 回ずつ解き、`ENOMOTO_MIP_NB_DIAG` の行 (stderr) を集める。

4 スレッド・60 秒、ソルバーは同時に 1 つだけ。問題ごとに logs/<name>.log (NB_DIAG の行と結果の JSON) を書く。
既にある問題は飛ばすので、中断しても同じコマンドで再開できる。

使い方: python analysis/basis_nb_diag_20261102/run.py [--only NAME ...]
"""
import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
WT = HERE.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
sys.path.insert(0, str(WT / "scripts"))
import probs  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument("--only", nargs="*")
ap.add_argument("--time-limit", type=float, default=60.0)
ap.add_argument("--threads", type=int, default=4)
a = ap.parse_args()
logs = HERE / "logs"
logs.mkdir(exist_ok=True)
for p in a.only or probs.PROBS:
    out = logs / f"{p}.log"
    if out.exists() and not a.only:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = dict(os.environ, ENOMOTO_MIP_THREADS=str(a.threads), ENOMOTO_MIP_NB_DIAG="1")
    cmd = [sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(a.time_limit), "--threads", str(a.threads)]
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=a.time_limit * 1.5 + 60, env=env)
        lines = [l for l in r.stderr.splitlines() if l.startswith("NB_DIAG")]
        res = next((l for l in reversed(r.stdout.splitlines()) if l.startswith("{")), json.dumps({"status": f"crash rc={r.returncode}"}))
    except subprocess.TimeoutExpired:
        lines, res = [], json.dumps({"status": "killed"})
    out.write_text("\n".join(lines + ["RESULT " + res]) + "\n")
    print(p, json.loads(res).get("status"), json.loads(res).get("time"), flush=True)
