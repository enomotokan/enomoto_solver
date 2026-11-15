"""58 問 (analysis/cbcimpl_20261029/probs.py) を既定 (def) と試作 (light1: ENOMOTO_MIP_SMALL3_SUBMIP_LIGHT=1) で 1 回ずつ、問題ごとに交互に解く
(4 スレッド・60 秒、ソルバーは同時に 1 つ)。結果は b58.jsonl に足す (中断しても再開できる)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029")); sys.path.insert(0, str(WT / "scripts"))
from probs import PROBS  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "light1": {"ENOMOTO_MIP_SMALL3_SUBMIP_LIGHT": "1"}}
OUT = D / "b58.jsonl"
done = {(j["problem"], j["tag"]) for j in map(json.loads, OUT.read_text().splitlines())} if OUT.exists() else set()
for p in PROBS:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for tag, e in CFGS.items():
        if (p, tag) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", **e)
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                               cwd=WT, env=env, capture_output=True, text=True, timeout=200)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        except Exception as ex:  # noqa: BLE001
            res = {"status": "crash", "error": str(ex)[:300], "time": 60.0}
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "tag": tag, "result": res, "at": time.strftime("%H:%M:%S")}) + "\n")
        print(f"{p:22s} {tag:7s} {res['status']:10s} {res['time']:6.1f}s obj={res.get('objective')}", flush=True)
