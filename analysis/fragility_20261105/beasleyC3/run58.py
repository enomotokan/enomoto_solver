"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で、既定 (base) と試作 (rsi) を問題ごとに交互に 1 回ずつ回す。
使い方: python run58.py 出力.jsonl  (途中から再開できる)"""
import json, os, subprocess, sys, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
WT = HERE.parents[2]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"base": {}, "rsi": {"ENOMOTO_MIP_ROBUST_LNSFREQ": "1", "ENOMOTO_MIP_ROBUST_IMPORT": "1"}}
out = Path(sys.argv[1])
done = set()
if out.exists():
    for l in out.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"]))
for i, p in enumerate(PROBS):
    order = ["base", "rsi"] if i % 2 == 0 else ["rsi", "base"]
    for cfg in order:
        if (p, cfg) in done:
            continue
        mps = rb.fetch(p, WT / ".miplib_cache")
        env = dict(os.environ, **CFGS[cfg])
        t0 = time.time()
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                               cwd=WT, env=env, capture_output=True, text=True, timeout=300)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(out, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": cfg, "wall": time.time() - t0, "result": res}) + "\n")
        print(p, cfg, res.get("status"), res.get("objective"), res.get("bound"), res.get("time"), flush=True)
