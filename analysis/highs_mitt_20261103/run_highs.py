"""Mittelmann 186 問 (analysis/cbcimpl_20261029/mitt_parts.json) を HiGHS (highspy) で 4 スレッド・60 秒で 1 回ずつ解く (2026-11-03)。
使い方: python analysis/highs_mitt_20261103/run_highs.py K N  (問題の並びを N 分担に分けた K 番目を解き、highs_sK.jsonl に足す。途中から再開できる)。
ENOMOTO の側は bench20 の回 114 (analysis/halve_submip_20261030/bench20_s*.jsonl の設定 p = 今の既定) と比べる (cmp.py)。"""
import json, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
k, n = int(sys.argv[1]), int(sys.argv[2])
probs = sorted({p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part})
mine = probs[k::n]
OUT = D / f"highs_s{k}.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
for p in mine:
    if p in done: continue
    for attempt in range(5):
        try:
            mps = rb.fetch(p, WT / ".miplib_cache"); break
        except Exception as e:  # noqa: BLE001
            time.sleep(30); err = e
    else:
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "result": {"status": "fetch_error", "error": str(err)[:200]}}) + "\n")
        continue
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-highs", str(mps), "--time-limit", "60", "--threads", "4"],
                           cwd=WT, capture_output=True, text=True, timeout=300)
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    except Exception as e:  # noqa: BLE001
        res = {"status": "crash", "error": str(e)[:300]}
    with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "result": res, "at": time.strftime("%H:%M:%S")}) + "\n")
