"""`ENOMOTO_MIP_LP_PROBE_AFTER_EXT` (試作) と既定を比べる。

問題ごと・設定ごと・回ごとに 1 つずつ順に解き (4 スレッド・60 秒、ソルバーは同時に 1 つ)、
logs/<設定>/<問題>_r<回>.log に NB_DIAG の行・根の LP と基底の行 (ENOMOTO_MIP_LOG の一部)・結果の JSON を書く。
既にあるログは飛ばすので、中断しても同じコマンドで再開できる。

使い方:
  python analysis/lp_probe_after_ext_20261102/run.py --cfg base ext base_nofj ext_nofj --reps 3 --only ns1116954 ...
  python analysis/lp_probe_after_ext_20261102/run.py --cfg base ext --reps 2 --rep0 10   # 58 問
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

CFGS = {"base": {}, "ext": {"ENOMOTO_MIP_LP_PROBE_AFTER_EXT": "1"}, "extr": {"ENOMOTO_MIP_LP_PROBE_AFTER_EXT": "r"},
        "base_nofj": {"ENOMOTO_MIP_FEAS_FJ_EARLY": "0"}, "ext_nofj": {"ENOMOTO_MIP_FEAS_FJ_EARLY": "0", "ENOMOTO_MIP_LP_PROBE_AFTER_EXT": "1"},
        "extr_nofj": {"ENOMOTO_MIP_FEAS_FJ_EARLY": "0", "ENOMOTO_MIP_LP_PROBE_AFTER_EXT": "r"}}
KEEP = ("NB_DIAG", "MIP: root LP", "MIP: LP probing", "MIP: restart ", "MIP: clique extension", "Presolve:", "MIP: presolve: rows")

ap = argparse.ArgumentParser()
ap.add_argument("--only", nargs="*")
ap.add_argument("--cfg", nargs="*", default=["base", "ext"])
ap.add_argument("--reps", type=int, default=1)
ap.add_argument("--rep0", type=int, default=0)
ap.add_argument("--time-limit", type=float, default=60.0)
ap.add_argument("--threads", type=int, default=4)
a = ap.parse_args()
for rep in range(a.rep0, a.rep0 + a.reps):
    for p in a.only or probs.PROBS:
        for c in a.cfg:
            d = HERE / "logs" / c
            d.mkdir(parents=True, exist_ok=True)
            out = d / f"{p}_r{rep}.log"
            if out.exists():
                continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = dict(os.environ, ENOMOTO_MIP_THREADS=str(a.threads), ENOMOTO_MIP_NB_DIAG="1", ENOMOTO_MIP_LOG="1", **CFGS[c])
            cmd = [sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(a.time_limit), "--threads", str(a.threads)]
            try:
                r = subprocess.run(cmd, capture_output=True, text=True, timeout=a.time_limit * 1.5 + 60, env=env)
                lines = [l[:400] for l in r.stderr.splitlines() if l.startswith(KEEP)]
                res = next((l for l in reversed(r.stdout.splitlines()) if l.startswith("{")), json.dumps({"status": f"crash rc={r.returncode}"}))
            except subprocess.TimeoutExpired:
                lines, res = [], json.dumps({"status": "killed"})
            out.write_text("\n".join(lines + ["RESULT " + res]) + "\n")
            j = json.loads(res)
            print(rep, p, c, j.get("status"), j.get("time"), j.get("objective"), flush=True)
