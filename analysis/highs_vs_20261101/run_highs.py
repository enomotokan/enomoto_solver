"""58 問 (analysis/cbcimpl_20261029/probs.py) を HiGHS (highspy) で 4 スレッド・60 秒で 1 回ずつ解き、highs.jsonl に足す (2026-11-01)。"""
import json, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029")); sys.path.insert(0, str(WT / "scripts"))
from probs import PROBS  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
OUT = D / "highs.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
for p in PROBS:
    if p in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-highs", str(mps), "--time-limit", "60", "--threads", "4"],
                           cwd=WT, capture_output=True, text=True, timeout=300)
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    except Exception as e:  # noqa: BLE001
        res = {"status": "crash", "error": str(e)[:300]}
    with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "result": res, "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, res.get("status"), res.get("time"), flush=True)
