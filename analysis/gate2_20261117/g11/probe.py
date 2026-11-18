# 候補 11 の見込みの確かめ: 既定と TREEEFF=0 (木の中の再スタートなし)・SEPA_MCF=0 を数問で比べる (60 秒・4 スレッド・種 1)
import json, os, subprocess, sys, re
from pathlib import Path
WT = Path("/home/user/enomoto_solver"); sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb
OUT = Path(__file__).parent / "probe.jsonl"
CF = {"def": {}, "notr": {"ENOMOTO_MIP_TREEEFF": "0"}, "nomcf": {"ENOMOTO_MIP_SEPA_MCF": "0"}}
JOBS = [(p, c) for p in ["mc11", "10teams", "neos-860300", "neos-3083819-nubu"] for c in ["def", "notr"]] + \
       [(p, c) for p in ["nu25-pr12", "h80x6320d"] for c in ["def", "nomcf"]]
for p, c in JOBS:
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CF[c]); env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1"})
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                        cwd=WT, env=env, capture_output=True, text=True)
    res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    nrs = len(re.findall(r"(?i)tree restart|in-tree restart|TRSTAT", pr.stderr))
    r = {"problem": p, "cfg": c, "status": res.get("status"), "obj": res.get("objective"), "time": round(res.get("time", 0), 1), "tr_lines": nrs}
    print(r, flush=True); OUT.open("a").write(json.dumps(r) + "\n")
