"""python run.py K: shards.json の K 番目を順に回し res_sK.jsonl に足す (続きから回せる)。4 スレッド。ログ (ENOMOTO_MIP_LOG=1) は logs/<問題>_<設定>_s<種>.log。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[2]
CFGS = {"def": {}, "vb": {"ENOMOTO_MIP_VB_IMPL": "1"}, "vbt": {"ENOMOTO_MIP_VB_IMPL": "t"}}
k = int(sys.argv[1])
mine = json.loads((D / "shards.json").read_text())["shards"][k]
OUT = D / f"res_s{k}.jsonl"; (D / "logs").mkdir(exist_ok=True)
done = {(r["problem"], r["cfg"], r["seed"]) for r in map(json.loads, OUT.open())} if OUT.exists() else set()
for p, cfg, seed, tl in mine:
    if (p, cfg, seed) in done: continue
    env = {k2: v for k2, v in os.environ.items() if not k2.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1")
    with open(D / "logs" / f"{p}_{cfg}_s{seed}.log", "w") as fe:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", str(tl), "--threads", "4"],
                            cwd=WT, env=env, stdout=subprocess.PIPE, stderr=fe, text=True)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash"}  # noqa: BLE001
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "tl": tl, "result": res, "shard": k, "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, cfg, seed, res.get("status"), res.get("time"), flush=True)
