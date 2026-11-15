"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で設定ごとに 1 回ずつ解き、bench58b.jsonl に足す (2026-11-06、nw04_bound)。
使い方: python bench58b.py <設定,...> <回>。設定: h1 (ENOMOTO_MIP_CUT_HARRIS2=1)・combb (analysis/halve_submip_20261030/run.py の CFGS)。
root_bound・bound を opt_table.json (リポジトリの過去の計測で最適と返した目的値の多数決) と比べ、越えたかを記す。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029")); sys.path.insert(0, str(WT / "analysis" / "halve_submip_20261030")); sys.path.insert(0, str(WT / "scripts"))
from probs import PROBS  # noqa: E402
from run import CFGS as HCFGS  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "h1": {"ENOMOTO_MIP_CUT_HARRIS2": "1"}, "combb": HCFGS["combb"]}
OPT = {p: v["opt"] for p, v in json.load(open(D / "opt_table.json")).items()}
OUT = D / "bench58b.jsonl"
cfgs, rep = sys.argv[1].split(","), int(sys.argv[2])
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
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", **CFGS[c])
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            res["restarts"] = sum(1 for l in r.stderr.splitlines() if re.match(r"MIP: restart \d+ \(", l))
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        o = OPT.get(p)
        tol = 1e-6 * max(1.0, abs(o)) if o is not None else 0
        over = {k: (o is not None and isinstance(res.get(k), (int, float)) and res[k] == res[k] and res[k] > o + tol) for k in ("root_bound", "bound")}
        wrong = o is not None and res.get("status") == "optimal" and abs(res.get("objective", o) - o) > 1e-6 * max(1.0, abs(o))
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "env": CFGS[c], "result": res, "opt": o, "over": over, "opt_differs": wrong,
                                "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), res.get("root_bound"), o, over, wrong, flush=True)
