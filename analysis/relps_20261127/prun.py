"""P (MIPLIB の 58 問、analysis/scipslow_20261119/shards_P.json) で RELPS を既定と比べる: 60 秒。
RUN_SPEC で (スレッド数, 種) の並びを選ぶ (例 "1:1,4:1,4:2")。RUN_CFG で試作の文字 (既定 u)。
1 スレッドは決定的なので種 1 だけ。python prun.py → prun.jsonl (途中から再開できる)"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
P = sorted({x[0] for s in json.loads((WT / "analysis/scipslow_20261119/shards_P.json").read_text())["shards"] for x in s})
OUT = D / "prun.jsonl"
rows = [json.loads(l) for l in OUT.read_text().splitlines()] if OUT.exists() else []
done = {(r["problem"], r["th"], r["seed"], r["cfg"]) for r in rows}
TL = 60
CFG = os.environ.get("RUN_CFG", "u")
SPEC = [tuple(map(int, x.split(":"))) for x in os.environ.get("RUN_SPEC", "1:1,4:1,4:2").split(",")]
def run(p, th, seed, cfg):
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_RELPS_DIAG": "1"})
    if cfg != "def": env["ENOMOTO_MIP_RELPS"] = cfg
    t0 = time.time()
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", str(th)],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try: out, err = pr.communicate(timeout=TL + 600)
    except subprocess.TimeoutExpired:
        pr.kill(); out, err = pr.communicate()
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    rl = [list(map(int, re.findall(r"\d+", l.split("RELPS:", 1)[1])))[:4] for l in err.splitlines() if "MIP: RELPS:" in l]
    return {"problem": p, "th": th, "seed": seed, "cfg": cfg, "result": res, "relps": rl[-1] if rl else None, "wall": round(time.time() - t0, 1)}
for th, seed in SPEC:
    for p in P:
        for cfg in (CFG, "def"):
            if (p, th, seed, cfg) in done: continue
            r = run(p, th, seed, cfg); rows.append(r)
            with open(OUT, "a") as f: f.write(json.dumps(r) + "\n")
            print(json.dumps({k: r[k] for k in ("problem", "th", "seed", "cfg", "relps")} | {"status": r["result"].get("status"), "time": r["result"].get("time"), "nodes": r["result"].get("nodes")}), flush=True)
print("PRUN_DONE", flush=True)
