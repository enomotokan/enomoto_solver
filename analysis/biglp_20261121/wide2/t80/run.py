"""ROOTLP3 の `e` の直し (最初の根では特異な基底を捨てない、7.10 節) の確かめ: fix (直し後の e) で neos-933966 (種 1 × 2 回・種 3)、
neos-1171448 (種 1 × 2 回)、sing44 (種 1、再スタートの後の根で捨てる行が残るか)。600 秒、4 スレッド。ログは t80/logs/、結果は t80/res.jsonl。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[3]
sys.path.insert(0, str(D.parent))
from cfg import CFGS  # noqa: E402
OUT = D / "res.jsonl"; (D / "logs").mkdir(exist_ok=True)
done = {(r["problem"], r["cfg"], r["seed"], r["rep"]) for r in map(json.loads, OUT.open())} if OUT.exists() else set()
RUNS = [("neos-933966", "fix", 1, 1), ("neos-933966", "fix", 1, 2), ("neos-933966", "fix", 3, 1),
        ("neos-1171448", "fix", 1, 1), ("neos-1171448", "fix", 1, 2), ("sing44", "fix", 1, 1)]
for prob, cfg, seed, rep in RUNS:
    if (prob, cfg, seed, rep) in done: continue
    mps = WT / ".miplib_cache" / f"{prob}.mps"
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1")
    log = D / "logs" / f"{prob}_{cfg}_s{seed}_r{rep}.log"
    with open(log, "w") as fe:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"],
                            cwd=WT, env=env, stdout=subprocess.PIPE, stderr=fe, text=True)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash"}  # noqa: BLE001
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": prob, "cfg": cfg, "seed": seed, "rep": rep, "result": res, "at": time.strftime("%H:%M:%S")}) + "\n")
    print(prob, cfg, seed, rep, res.get("status"), res.get("time"), flush=True)
