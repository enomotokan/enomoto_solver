"""手間の単位の数えの負荷 (analysis/timedep_20261114.md の 11.3 節): python ticks_overhead.py 前の版の python 今の版の python 問題,... 回数
束 cand1115c、1 スレッド、手間の上限 (tool/wl_T.json、同じ道になる) で、2 つの版を交互に解き、子の CPU 秒 (user + sys) を比べる。CPU 0 に固定。"""
import json, os, resource, subprocess, sys, importlib.util
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sp = importlib.util.spec_from_file_location("cfg1115", WT / "analysis/cand1115_20261115/cfg.py"); m = importlib.util.module_from_spec(sp); sp.loader.exec_module(m)
py_old, py_new, probs, reps = sys.argv[1], sys.argv[2], sys.argv[3].split(","), int(sys.argv[4])
wl = json.loads((D / "tool/wl_T.json").read_text())
CODE = """
import json, sys
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
mm, _, _ = bh._build_our_model(lp)
r = mm.solve(None, False, 600.0, None, None, 1, True)
print("RES " + json.dumps({k: r.get(k) for k in ("status", "nodes", "lp_iterations")}))
"""
for p in probs:
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(m.CAND1115C); env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_WORK_LIMIT": str(wl[p])})
    out = {"old": [], "new": []}; keys = set()
    for r in range(reps):
        for tag, py in (("old", py_old), ("new", py_new)):
            u0 = resource.getrusage(resource.RUSAGE_CHILDREN)
            pr = subprocess.run(["taskset", "-c", "0", py, "-c", CODE, str(WT / ".miplib_cache" / f"{p}.mps")], env=env, cwd="/tmp", capture_output=True, text=True)
            u1 = resource.getrusage(resource.RUSAGE_CHILDREN)
            res = [l for l in pr.stdout.splitlines() if l.startswith("RES ")]
            keys.add((tag, res[-1] if res else "none"))
            out[tag].append(u1.ru_utime + u1.ru_stime - u0.ru_utime - u0.ru_stime)
    o, n = sorted(out["old"]), sorted(out["new"])
    print(json.dumps({"problem": p, "old": [round(x, 2) for x in o], "new": [round(x, 2) for x in n], "ratio_median": round(n[len(n) // 2] / o[len(o) // 2], 4), "results": sorted(keys)}), flush=True)
