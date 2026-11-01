"""58 問 (負けている 21 問 + MIPLIB 40 問) を HiGHS 1.15.1 で解く (4 スレッド・60 秒、クラウドの 4 コアの機械、2026-10-30)。
enomoto の計測 (analysis/halve_submip_20261030/) と同じ種類の機械で、HiGHS が解ける問題を確かめるため。結果は highs.jsonl。"""
import json, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
OUT = D / "highs.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
for p in PROBS:
    if p in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-highs", str(mps), "--time-limit", "60", "--threads", "4"],
                       cwd=WT, capture_output=True, text=True, timeout=400)
    try:
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    except Exception as e:  # noqa: BLE001
        res = {"status": "crash", "error": (str(e) + r.stderr[-300:])}
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "result": res, "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, res.get("status"), res.get("time"), flush=True)
