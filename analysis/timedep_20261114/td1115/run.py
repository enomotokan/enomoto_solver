"""分担を回す: python run.py K (shards.json の K 番目を 4 スレッドで順に解き、res_sK.jsonl に足す。途中から再開できる)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
K = int(sys.argv[1])
mine = json.loads((D / "shards.json").read_text())["shards"][K]
OUT = D / f"res_s{K}.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip(): r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
for p, c, s, tl in mine:
    if (p, c, s) in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[c]); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(s)
    t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=tl * 2.5 + 300)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": c, "seed": s, "tl": tl, "result": res, "wall": round(time.time() - t0, 1), "shard": K}) + "\n")
    print(p, s, c, res.get("status"), res.get("time"), flush=True)
