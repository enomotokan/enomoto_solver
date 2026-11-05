"""run58.py で差の出た問題を、設定ごとに回を重ねて解き直し res58_re.jsonl に足す (揺れか害かの見分け、2026-11-05)。
使い方: python rerun.py <回の始め> <回の終わり> <問題,...> [設定,... 既定 def,old]  (回ごとに設定の順を入れ替える)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
CFGS = {"def": {}, "old": {"ENOMOTO_MIP_POSTSOLVE_CLIP": "0", "ENOMOTO_MIP_CONFLICT_INT_TOL": "0"},
    # 最初に試した直し (整数の余裕を 1/2): neos5・physiciansched6-2 が遅くなったので既定にしなかった
    "half": {"ENOMOTO_MIP_CONFLICT_INT_TOL": "h"}}
r0, r1, probs = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3].split(",")
cfgs = sys.argv[4].split(",") if len(sys.argv) > 4 else ["def", "old"]
for rep in range(r0, r1):
    for p in probs:
        for c in (cfgs if rep % 2 == 0 else cfgs[::-1]):
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(ENOMOTO_MIP_THREADS="4", **CFGS[c])
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            with open(D / "res58_re.jsonl", "a") as f:
                f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": res}) + "\n")
            print(p, c, rep, res["status"], round(res["time"], 1), res["objective"], flush=True)
