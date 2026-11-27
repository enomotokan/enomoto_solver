"""3 回目: OBJSHAVE=djc で同じことをする (pharm3.jsonl)。以下は 1 回目の説明:
P (MIPLIB の 58 問、analysis/scipslow_20261119/shards_P.json) で OBJSHAVE の害を測る: 4 スレッド・種 1〜3・60 秒。
各 (問題, 種) で先に shave を回し、試作が働いた (ログに「OBJSHAVE: started」) ときだけ既定も回す (働かなければ既定と同じ設定)。
python pharm.py → pharm.jsonl (途中から再開できる)。PYTHONPATH は呼ぶ側で渡す"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
P = sorted({x[0] for s in json.loads((WT / "analysis/scipslow_20261119/shards_P.json").read_text())["shards"] for x in s})
OUT = D / "pharm3.jsonl"
rows = [json.loads(l) for l in OUT.read_text().splitlines()] if OUT.exists() else []
done = {(r["problem"], r["seed"], r["cfg"]) for r in rows}
TL = 60
def run(p, seed, cfg):
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_MIP_LOG": "1"})
    if cfg == "shave": env["ENOMOTO_MIP_OBJSHAVE"] = "djc"
    t0 = time.time()
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try: out, err = pr.communicate(timeout=TL + 600)
    except subprocess.TimeoutExpired:
        pr.kill(); out, err = pr.communicate()
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    sh = [l for l in err.splitlines() if "OBJSHAVE" in l]
    return {"problem": p, "seed": seed, "cfg": cfg, "result": res, "active": any("OBJSHAVE: role prepared" in l for l in sh),
            "proofs": sum("received" in l for l in sh), "roles": sum("role on" in l for l in sh), "accepted": sum("accepted true" in l for l in sh), "fj_found": sum("FJ/Local-MIP found" in l for l in sh), "summary": next((l for l in sh if "tries" in l), None), "wall": round(time.time() - t0, 1)}
for seed in (1, 2, 3):
    for p in P:
        for cfg in ("shave", "def"):
            if (p, seed, cfg) in done: continue
            if cfg == "def":
                sh = [r for r in rows if r["problem"] == p and r["seed"] == seed and r["cfg"] == "shave"]
                if sh and not sh[0]["active"]: continue
            r = run(p, seed, cfg); rows.append(r)
            with open(OUT, "a") as f: f.write(json.dumps(r) + "\n")
            print(json.dumps({k: r[k] for k in ("problem", "seed", "cfg", "active", "proofs", "roles", "accepted")} | {"status": r["result"].get("status"), "time": r["result"].get("time")}), flush=True)
print("PHARM_DONE", flush=True)
