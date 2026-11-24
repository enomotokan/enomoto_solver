"""SCIP (pyscipopt 6.3.0 = SCIP 10.0) で 1 回解く: python scip_one.py 問題 名札 [名=値 ...]
逐次・時間の上限 CR_TL (既定 120 秒)。根の下界 (getDualboundRoot)・根の分離ラウンド・ノード・時間を scip.jsonl に足す。ログは logs/scip_<問題>_<名札>.log。"""
import json, os, sys, time
from pathlib import Path
import pyscipopt
D = Path(__file__).resolve().parent; WT = D.parents[1]
p, tag = sys.argv[1], sys.argv[2]; extra = sys.argv[3:]
m = pyscipopt.Model(); (D / "logs").mkdir(exist_ok=True); LOG = D / "logs" / f"scip_{p}_{tag}.log"; m.setLogfile(str(LOG)); m.hideOutput()
m.setParam("limits/time", float(os.environ.get("CR_TL", "120")))
for e in extra:
    k, v = e.split("=", 1); cur = m.getParam(k)
    m.setParam(k, type(cur)(v) if not isinstance(cur, bool) else v.lower() in ("1", "true"))
m.readProblem(str(WT / ".miplib_cache" / f"{p}.mps"))
t = time.perf_counter(); m.optimize(); t = time.perf_counter() - t
st = D / "logs" / f"scip_{p}_{tag}.stat"; m.writeStatistics(str(st))
row = {"problem": p, "tag": tag, "extra": extra, "status": m.getStatus(), "root_dual": m.getDualboundRoot(), "dual": m.getDualbound(),
       "primal": m.getPrimalbound() if m.getNSols() else None, "nodes": m.getNNodes(), "time": round(t, 2)}
print(json.dumps(row))
with open(D / "scip.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
