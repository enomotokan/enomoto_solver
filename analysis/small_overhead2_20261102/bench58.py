"""small_overhead2 (2026-11-02): 58 問 (analysis/cbcimpl_20261029/probs.py) を、既定 (def)・FJSTOP・FJSTOP+SUBLIGHT で 1 回ずつ
(問題ごとに 3 設定を続けて) 解く。4 スレッド・60 秒、ENOMOTO_MIP_LOG=1。結果は bench58.jsonl (途中から続けられる)、ログは logs58/ に gzip で。"""
import gzip, json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
CFGS = {"def": {}, "fjstop": {"ENOMOTO_MIP_SMALL2_FJSTOP": "1"},
        "fjsl": {"ENOMOTO_MIP_SMALL2_FJSTOP": "1", "ENOMOTO_MIP_SMALL2_SUBLIGHT": "1"}}
OUT = D / "bench58.jsonl"
done = {(r["problem"], r["cfg"]) for r in map(json.loads, OUT.read_text().splitlines())} if OUT.exists() else set()
(D / "logs58").mkdir(exist_ok=True)
for p in PROBS:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c, e in CFGS.items():
        if (p, c) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", **e)
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                               cwd=WT, env=env, capture_output=True, text=True, timeout=300)
            with gzip.open(D / "logs58" / f"{p}_{c}.log.gz", "wt") as f: f.write(r.stderr + r.stdout)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        except Exception as ex:  # noqa: BLE001
            res = {"status": "crash", "time": -1, "error": str(ex)[:300]}
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "cfg": c, "result": res}) + "\n")
        print(f"{p:24s} {c:7s} {res['status']:12s} {res['time']:6.1f}s obj={res.get('objective')}", flush=True)
