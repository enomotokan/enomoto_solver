"""mem3・forceq・fstop の既定化の確かめ (2026-11-04): 既定 (def) と戻した形 (preall4) で数問を解き、試作が働いた印のログの行を数える。
使い方: python analysis/mem3_default_20261104/check.py > analysis/mem3_default_20261104/check.out"""
import os, subprocess, sys, json
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(WT / "analysis/halve_submip_20261030"))
sys.path.insert(0, str(WT / "scripts"))
from run import CFGS  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
KEYS = ["memc (all threads)", "MEMCAP", "forceq", "FORCEQ", "find thread asked to stop"]
for p, tl in [("neos-1445765", 40), ("qiu", 30), ("neos-1456979", 20)]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c in ["def", "preall4"]:
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(CFGS[c]); env["ENOMOTO_MIP_LOG"] = "1"; env["ENOMOTO_MIP_THREADS"] = "4"
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                           cwd=WT, env=env, capture_output=True, text=True)
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        hits = {k: sum(1 for l in r.stderr.splitlines() if k in l) for k in KEYS}
        print(p, c, res["status"], round(res["time"], 1), res.get("objective"), hits, flush=True)
