"""根のラウンドの得・損の診断 (ENOMOTO_MIP_CUTSTOP_LOG=1 の CUTTK・TREETK の行) を 1 スレッド・種 1 で集める。
設定は def と long (analysis/rootstop_20261121/cfg.py の LONG)、mc11 は CMIRGEN=u (cmu・cmu_long) も。上限は P の問題 120 秒、swath1・beasleyC3 も 120 秒。
python diag.py → diag.jsonl (結果) と logs/<問題>_<設定>.log"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/rootstop_20261121"))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import LONG  # noqa: E402
BAD = ["neos5", "mc11", "neos17", "pk1", "timtab1", "mik-250-20-75-4", "roll3000"]
GOOD = ["noswot", "10teams", "qiu", "eil33-2", "neos8", "swath1", "beasleyC3"]
CF = {"def": {}, "long": LONG}
RUNS = [(p, c) for p in BAD + GOOD for c in CF] + [("mc11", "cmu"), ("mc11", "cmu_long")]
CF["cmu"] = {"ENOMOTO_MIP_CMIRGEN": "u"}; CF["cmu_long"] = {**LONG, "ENOMOTO_MIP_CMIRGEN": "u"}
OUT = D / "diag.jsonl"
done = {(r["problem"], r["cfg"]) for r in map(json.loads, OUT.open())} if OUT.exists() else set()
for p, c in RUNS:
    if (p, c) in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CF[c]); env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_CUTSTOP_LOG="1")
    with open(D / "logs" / f"{p}_{c}.log", "w") as fe:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "120", "--threads", "1"],
                            cwd=WT, env=env, stdout=subprocess.PIPE, stderr=fe, text=True)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash"}  # noqa: BLE001
    with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "cfg": c, "result": res}) + "\n")
    print(p, c, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
