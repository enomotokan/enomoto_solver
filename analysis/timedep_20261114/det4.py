"""4 スレッドの決定性の確かめ (analysis/timedep_20261114.md の 12.4 節): python det4.py OUT.jsonl TAG 問題,... 種 [--slow] [K=V ...]
1 問ずつ順に 4 スレッドで解く (taskset なし、手間の上限なし、時間の上限 60 秒)。束は cand1116 + K=V。
--slow: 4 つの CPU で `yes` を回して遅い機械を真似る。行: problem, tag, seed, status, objective, nodes, lp_iterations, time。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
import importlib.util as _iu
_sp = _iu.spec_from_file_location("cfg1116", WT / "analysis/cand1116_20261115/cfg.py"); _m = _iu.module_from_spec(_sp); _sp.loader.exec_module(_m)
a = sys.argv[1:]
out, tag, probs, seed = Path(a[0]), a[1], a[2].split(","), int(a[3])
slow = "--slow" in a; kv = [x for x in a[4:] if "=" in x]
CODE = """
import json, sys, time
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter(); r = m.solve(None, False, 60.0, None, None, 4, True); t = time.perf_counter() - t
print("TOOLRES " + json.dumps({k: r.get(k) for k in ("status", "objective", "best_bound", "nodes", "lp_iterations")} | {"time": t}), flush=True)
"""
for p in probs:
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(_m.CAND1116); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(seed)
    for x in kv:
        k, v = x.split("=", 1); env[k] = v
    hogs = [subprocess.Popen(["taskset", "-c", str(c), "yes"], stdout=subprocess.DEVNULL) for c in range(4)] if slow else []
    pr = subprocess.run([sys.executable, "-c", CODE, str(WT / ".miplib_cache" / f"{p}.mps")], env=env, cwd=WT, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    for h in hogs: h.kill(); h.wait()
    res = [l for l in pr.stdout.splitlines() if l.startswith("TOOLRES ")]
    r = json.loads(res[-1][8:]) if res else {"status": "crash"}
    with open(out, "a") as f: f.write(json.dumps({"problem": p, "tag": tag, "seed": seed, "env": kv} | r) + "\n")
    print(p, tag, r.get("status"), r.get("nodes"), r.get("lp_iterations"), round(r.get("time") or 0, 1), flush=True)
