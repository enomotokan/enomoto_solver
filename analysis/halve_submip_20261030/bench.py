"""58 問 (負けている 21 問 + MIPLIB 40 問) を 4 スレッド・60 秒で設定ごとに解き、bench.jsonl に足す (2026-10-30)。
使い方: python bench.py <設定名,...> <回> (設定は run.py の CFGS。設定の順は回ごとに逆にする)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
sys.path.insert(0, str(D))
from probs import PROBS  # noqa: E402
from run import CFGS  # noqa: E402  (run.py は引数なしで読むと何もしない)
OUT = D / "bench.jsonl"
cfgs, rep = sys.argv[1].split(","), int(sys.argv[2])
if rep % 2:
    cfgs = cfgs[::-1]
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for p in PROBS:
    for c in cfgs:
        if (p, c, rep) in done:
            continue
        env = dict(os.environ, ENOMOTO_MIP_THREADS="4", **CFGS[c])
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=300)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": res, "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), flush=True)
