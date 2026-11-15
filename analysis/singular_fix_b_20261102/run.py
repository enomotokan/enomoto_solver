"""試作 `ENOMOTO_MIP_POOL_BASIS_FIX` (共有の待ち行列のノードの基底を非特異に直す) を既定と比べる。

設定ごと・回ごとに logs/<設定>/<問題>.r<回>.log (NB_DIAG と POOL_BASIS_FIX の行と結果の JSON) を書く。
4 スレッド・60 秒、`ENOMOTO_MIP_NB_DIAG=1`、ソルバーは同時に 1 つだけ。既にあるログは飛ばすので同じコマンドで再開できる。

使い方:
  python analysis/singular_fix_b_20261102/run.py --set six --reps 3 --configs base fix keep
  python analysis/singular_fix_b_20261102/run.py --set all58 --reps 1 --configs base fix
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

SIX = ["neos5", "misc07", "noswot", "neos-911970", "swath1", "eil33-2", "roll3000", "timtab1"]
CONFIGS = {
    "base": {},
    "fix": {"ENOMOTO_MIP_POOL_BASIS_FIX": "1"},
    # 参考: 余りがあれば受け取った基底を使わず今の LP の基底のまま解く既存の試作
    "keep": {"ENOMOTO_MIP_TREE_POOL_BASIS": "1"},
}

ap = argparse.ArgumentParser()
ap.add_argument("--set", choices=["six", "all58"], default="six")
ap.add_argument("--only", nargs="*")
ap.add_argument("--reps", type=int, default=1)
ap.add_argument("--configs", nargs="*", default=["base", "fix"])
ap.add_argument("--time-limit", type=float, default=60.0)
ap.add_argument("--threads", type=int, default=4)
a = ap.parse_args()
names = a.only or (SIX if a.set == "six" else probs.PROBS)
for rep in range(1, a.reps + 1):
    for p in names:
        for c in a.configs:
            d = HERE / ("logs" if a.set == "six" else "logs58") / c
            d.mkdir(parents=True, exist_ok=True)
            out = d / f"{p}.r{rep}.log"
            if out.exists():
                continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = dict(os.environ, ENOMOTO_MIP_THREADS=str(a.threads), ENOMOTO_MIP_NB_DIAG="1", **CONFIGS[c])
            cmd = [sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(a.time_limit), "--threads", str(a.threads)]
            try:
                r = subprocess.run(cmd, capture_output=True, text=True, timeout=a.time_limit * 1.5 + 60, env=env)
                lines = [l for l in r.stderr.splitlines() if l.startswith(("NB_DIAG", "POOL_BASIS_FIX"))]
                res = next((l for l in reversed(r.stdout.splitlines()) if l.startswith("{")), json.dumps({"status": f"crash rc={r.returncode}"}))
            except subprocess.TimeoutExpired:
                lines, res = [], json.dumps({"status": "killed"})
            out.write_text("\n".join(lines + ["RESULT " + res]) + "\n")
            j = json.loads(res)
            print(rep, p, c, j.get("status"), j.get("time"), j.get("nodes"), flush=True)
