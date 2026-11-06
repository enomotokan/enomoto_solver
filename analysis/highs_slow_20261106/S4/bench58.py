"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で設定ごとに 1 回ずつ解き、bench58.jsonl に足す (2026-11-06、S4)。
使い方: python bench58.py <設定名,...> <回> (設定は cfgs58.json の鍵と def。設定の順は回ごとに逆、ソルバーは同時に 1 つ。種は ENOMOTO_T_MIP_SEED=回)。
ENOMOTO_MIP_LOG=1 で解き、試作が働いた印 (ログの S4_ の行の数) を結果に足す (ログは残さない)。途中は scratchpad に書き、終わってから写す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029")); sys.path.insert(0, str(WT / "scripts"))
from probs import PROBS  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, **json.loads((D / "cfgs58.json").read_text())}
OUT = Path(os.environ.get("BENCH58_OUT", str(D / "bench58.jsonl")))
cfgs, rep = sys.argv[1].split(","), int(sys.argv[2])
if rep % 2:
    cfgs = cfgs[::-1]
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for p in PROBS:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c in cfgs:
        if (p, c, rep) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=str(rep), **CFGS[c])
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=300)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            res["s4"] = {k: r.stderr.count(k) for k in ("S4_TINYCUT", "S4_BGZERO", "S4_RINSRATE", "S4_TINYGAP")}
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "env": CFGS[c], "result": res, "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), res.get("s4"), flush=True)
