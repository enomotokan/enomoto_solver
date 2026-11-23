"""相手の根の下界を取る: python rootcmp.py → rootcmp.jsonl (1 問 1 ソルバー 1 行、途中から再開できる)。
SCIP は limits/nodes = 1 (前処理・根の LP・根のカットまで)、HiGHS は mip_max_nodes = 1 で解き、根の後の下界 (と最初の LP の値) を残す。
対象は compare.txt の「今も 2 倍以上負け」の 27 問。上限 600 秒、種 1、1 本ずつ。当方の根の下界は res_s*.jsonl の root_bound (ours_t1)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
probs = open(D / "compare.txt", encoding="utf-8").read().split("今も 2 倍以上負け (どれかの組): 27 問: ")[1].split()
OUT = D / "rootcmp.jsonl"
done = {(r["problem"], r["solver"]) for r in (json.loads(l) for l in open(OUT)) } if OUT.exists() else set()
SCIP = r'''
import sys, json
from pyscipopt import Model
m = Model(); m.hideOutput(); m.setParam("limits/time", 600); m.setParam("randomization/randomseedshift", 1); m.setParam("limits/nodes", 1)
m.readProblem(sys.argv[1]); m.optimize()
fl = None
try:
    fl = m.getFirstLpObjVal() if hasattr(m, "getFirstLpObjVal") else None
except Exception: pass
print(json.dumps({"root": m.getDualboundRoot(), "first_lp": fl, "time": m.getSolvingTime(), "status": m.getStatus()}))
'''
HIGHS = r'''
import sys, json, time, highspy
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("time_limit", 600.0); h.setOptionValue("mip_rel_gap", 0.0)
h.setOptionValue("random_seed", 1); h.setOptionValue("mip_max_nodes", 1)
h.readModel(sys.argv[1]); t = time.time(); h.run()
i = h.getInfo()
print(json.dumps({"root": i.mip_dual_bound, "time": time.time() - t, "status": h.modelStatusToString(h.getModelStatus()), "nodes": i.mip_node_count}))
'''
for p in probs:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for solver, code in (("scip", SCIP), ("highs", HIGHS)):
        if (p, solver) in done: continue
        t0 = time.time()
        try:
            r = subprocess.run([sys.executable, "-c", code, str(mps)], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, timeout=900)
            res = json.loads(r.stdout.strip().splitlines()[-1])
        except Exception as e:  # noqa: BLE001
            res = {"error": str(e)[:200]}
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "solver": solver, "result": res, "wall": round(time.time() - t0, 1)}) + "\n")
        print(p, solver, res, flush=True)
