"""根だけ (node_limit 1) を当方の既定 + 環境変数で解き、ログを logs/root_<問題>_<札>.log、結果を root.jsonl に足す。
python root.py <問題> <札> [K=V ...]   (4 スレッド・600 秒・ENOMOTO_MIP_LOG=1、行の頭に経過秒)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
p, tag = sys.argv[1], sys.argv[2]; kv = [a for a in sys.argv[3:] if "=" in a]
OCODE = """
import json, sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
mo, _, _ = bh._build_our_model(lp)
t = time.perf_counter(); r = mo.solve(None, False, 600.0, None, 1, 4, True); t = time.perf_counter() - t
print("RES " + json.dumps({k: r.get(k) for k in ("status", "objective", "best_bound", "nodes", "lp_iterations", "root_bound")} | {"time": t}))
"""
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_T_MIP_SEED": "1"})
for x in kv:
    k, v = x.split("=", 1); env[k] = v
lf = D / "logs" / f"root_{p}_{tag}.log"
t0 = time.time(); out = []
pr = subprocess.Popen([sys.executable, "-c", OCODE, str(WT / ".miplib_cache" / f"{p}.mps")], env=env, cwd=WT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
with open(lf, "w") as f:
    for l in pr.stdout:
        f.write(f"[{time.time() - t0:7.2f}] {l}"); out.append(l)
res = [l for l in out if l.startswith("RES ")]
r = json.loads(res[-1][4:]) if res else {"status": "crash"}
row = {"problem": p, "tag": tag, "env": kv} | r
with open(D / "root.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
print(json.dumps(row), flush=True)
