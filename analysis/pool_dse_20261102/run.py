"""試作 `ENOMOTO_MIP_POOL_DSE` (共有の待ち行列で渡すノードに DSE 重みも付ける) を既定・`ENOMOTO_MIP_POOL_BASIS_FIX` と比べる。

設定ごと・回ごとに logs/<設定>/<問題>.r<回>.log (NB_DIAG・POOL_BASIS_FIX・POOL_DSE の行と結果の JSON) を書き、
ENOMOTO_MIP_LOG=1 の標準エラーの全文を logs/<設定>/<問題>.r<回>.err に書く。
4 スレッド・60 秒、`ENOMOTO_MIP_NB_DIAG=1`、ソルバーは同時に 1 つだけ。既にあるログは飛ばすので同じコマンドで再開できる。

使い方:
  python analysis/pool_dse_20261102/run.py --reps 2 --configs base fix dse both
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

SIX = ["neos5", "misc07", "noswot", "neos-911970", "swath1", "eil33-2", "roll3000", "mc11"]
CONFIGS = {
    "base": {},
    "fix": {"ENOMOTO_MIP_POOL_BASIS_FIX": "1"},
    "dse": {"ENOMOTO_MIP_POOL_DSE": "1"},
    "both": {"ENOMOTO_MIP_POOL_BASIS_FIX": "1", "ENOMOTO_MIP_POOL_DSE": "1"},
    # 重みのない基底変数を NaN (厳密な値を埋める) にする形
    "dsen": {"ENOMOTO_MIP_POOL_DSE": "n"},
    "bothn": {"ENOMOTO_MIP_POOL_BASIS_FIX": "1", "ENOMOTO_MIP_POOL_DSE": "n"},
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
            env = dict(os.environ, ENOMOTO_MIP_THREADS=str(a.threads), ENOMOTO_MIP_NB_DIAG="1", ENOMOTO_MIP_LOG="1", **CONFIGS[c])
            cmd = [sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(a.time_limit), "--threads", str(a.threads)]
            try:
                r = subprocess.run(cmd, capture_output=True, text=True, timeout=a.time_limit * 1.5 + 60, env=env)
                lines = [l for l in r.stderr.splitlines() if l.startswith(("NB_DIAG", "POOL_BASIS_FIX", "POOL_DSE"))]
                (d / f"{p}.r{rep}.err").write_text(r.stderr)
                res = next((l for l in reversed(r.stdout.splitlines()) if l.startswith("{")), json.dumps({"status": f"crash rc={r.returncode}"}))
            except subprocess.TimeoutExpired:
                lines, res = [], json.dumps({"status": "killed"})
            out.write_text("\n".join(lines + ["RESULT " + res]) + "\n")
            j = json.loads(res)
            print(rep, p, c, j.get("status"), j.get("time"), j.get("nodes"), flush=True)
