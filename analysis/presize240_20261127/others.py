"""PRESIZE240 7 節: HiGHS (1・4 スレッド) と SCIP (1 スレッド) で ex9・ex10 を 600 秒で解く。出力: others.jsonl"""
import json, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
SCIP = r'''
import json, sys, time, pyscipopt
m = pyscipopt.Model(); m.hideOutput(); m.setParam("limits/time", 600.0); m.setParam("limits/gap", 0.0); m.readProblem(sys.argv[1])
t0 = time.time(); m.optimize(); t = time.time() - t0
print(json.dumps({"status": m.getStatus(), "time": round(t, 2), "obj": m.getObjVal() if m.getNSols() else None, "nodes": m.getNNodes()}))
'''
for p in sys.argv[1].split(","):
    mps = WT / ".miplib_cache" / f"{p}.mps"
    sys.path.insert(0, str(WT / "scripts")); import run_miplib_benchmark as rb; rb.fetch(p, WT / ".miplib_cache")
    for who, cmd in (("highs_t1", [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-highs", str(mps), "--time-limit", "600", "--threads", "1"]),
                     ("highs_t4", [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-highs", str(mps), "--time-limit", "600", "--threads", "4"]),
                     ("scip_t1", [sys.executable, "-c", SCIP, str(mps)])):
        t0 = time.time()
        pr = subprocess.run(cmd, cwd=WT, capture_output=True, text=True, timeout=900)
        res = next((json.loads(l) for l in reversed(pr.stdout.splitlines()) if l.startswith("{")), {"status": "crash", "err": pr.stderr[-300:]})
        r = {"problem": p, "solver": who, "result": res, "wall": round(time.time() - t0, 1)}
        with open(D / "others.jsonl", "a") as f: f.write(json.dumps(r) + "\n")
        print(json.dumps(r), flush=True)
