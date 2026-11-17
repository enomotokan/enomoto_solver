"""eil33-2 の種 1 の落とし (README の 4 節) が揺れかを見る: c18 と c18_sig を 4 スレッド・60 秒で、種 1 を 3 回ずつと種 5〜8 を 1 回ずつ
交互に解き、eil_rep.jsonl に足す。python eil_rep.py"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[2]
sys.path.insert(0, str(D)); from cfg import CFGS  # noqa: E402
mps = WT / ".miplib_cache" / "eil33-2.mps"
jobs = [(1, r) for r in range(3)] + [(s, 0) for s in (5, 6, 7, 8)]
for seed, rep in jobs:
    for cfg in ("c18", "c18_sig") if (seed + rep) % 2 == 0 else ("c18_sig", "c18"):
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(seed)
        out = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                             cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True).stdout
        r = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
        row = {"cfg": cfg, "seed": seed, "rep": rep, "status": r.get("status"), "time": r.get("time"), "objective": r.get("objective")}
        with open(D / "eil_rep.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
        print(row, flush=True)
