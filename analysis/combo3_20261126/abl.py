"""c3 の部品の切り分け (P の前測りの 4 スレッドの遅れ、2026-11-26): python analysis/combo3_20261126/abl.py OUT.jsonl 問題 スレッド数 種 CFG[,CFG...] [--tl 秒]
設定: def・c2c (combo2 の C2C)・c3・c3-X (c3 から部品 X を外す。X は cfg.py の PARTS の名前)。同じ (問題, 設定, スレッド, 種) の行があれば飛ばす。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import C2C, C3, c3_without  # noqa: E402


def cfg_env(c):
    if c == "def": return {}
    if c == "c2c": return dict(C2C)
    if c == "c3": return dict(C3)
    if c.startswith("c3-"): return c3_without(*c[3:].split("-"))
    raise ValueError(c)


args = sys.argv[1:]
TL = 60.0
if "--tl" in args:
    i = args.index("--tl"); TL = float(args[i + 1]); del args[i:i + 2]
OUT = Path(args[0]); prob, th, seed = args[1], args[2], int(args[3]); cfgs = args[4].split(",")
mps = rb.fetch(prob, WT / ".miplib_cache")
done = {(r["problem"], r["cfg"], r["threads"], r["seed"]) for r in map(json.loads, OUT.read_text().splitlines())} if OUT.exists() else set()
for c in cfgs:
    if (prob, c, th, seed) in done: continue
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(cfg_env(c)); env.update(ENOMOTO_MIP_THREADS=th, ENOMOTO_T_MIP_SEED=str(seed))
    t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", th],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 900)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "rc": pr.returncode}  # noqa: BLE001
    row = {"problem": prob, "cfg": c, "threads": th, "seed": seed, "tl": TL, "result": res, "wall": round(time.time() - t0, 1)}
    with OUT.open("a") as f: f.write(json.dumps(row) + "\n")
    print(prob, th, seed, c, res.get("status"), round(res.get("time") or 0, 2), res.get("nodes"), flush=True)
