"""計算が変わる問題 (内点法が根の LP に勝つ) で、4 スレッドの種 2・3 を def と X で回し直す。python full4.py X TL 問題... >> full4.jsonl
ログ (ENOMOTO_MIP_LOG・XOLAZY_LOG) は logs4/PROB_sSEED_CFG.log.gz"""
import gzip, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D.parent))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
X, TL = sys.argv[1], sys.argv[2]
(D / "logs4").mkdir(exist_ok=True)
for p in sys.argv[3:]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for s in (2, 3):
        for c in ("def", X):
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(CFGS[c]); env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_XOLAZY_LOG": "1", "ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": str(s)})
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", "4"],
                                cwd=WT, env=env, capture_output=True, text=True, timeout=int(TL) + 600)
            err = pr.stderr
            with gzip.open(D / "logs4" / f"{p}_s{s}_{c}.log.gz", "wt") as f: f.write(err)
            res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
            xo = [json.loads(m) for m in re.findall(r"XOLAZY (\{.*\"ev\":\"rootlp\".*\})", err)]
            rl = re.findall(r"MIP: root LP (\S+) \((\d+) iters", err)
            row = {"problem": p, "seed": s, "cfg": c, "status": res.get("status"), "time": res.get("time"), "objective": res.get("objective"), "bound": res.get("bound"),
                   "nodes": res.get("nodes"), "root_bound": res.get("root_bound"), "winners": [x.get("winner") for x in xo], "rootlp": rl[:2]}
            with open(D / "full4.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
            print(p, s, c, row["status"], round(row["time"] or 0, 1), row["objective"], row["bound"], row["root_bound"], row["winners"], rl[:1], flush=True)
