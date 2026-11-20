"""P2-INF: SCIP の機能を 1 つ切って 1 問を解く (何が効いているかの切り分け)。python scip_abl.py <mps> <秒> <名前> [param=value ...]"""
import sys
from pyscipopt import Model
m = Model(); m.hideOutput()
m.readProblem(sys.argv[1]); m.setParam("limits/time", float(sys.argv[2])); m.setParam("parallel/maxnthreads", 1)
for kv in sys.argv[4:]:
    k, v = kv.split("=", 1)
    v = {"TRUE": True, "FALSE": False}.get(v, v)
    try: v = int(v)
    except (ValueError, TypeError): pass
    m.setParam(k, v)
m.optimize()
print(sys.argv[3], m.getStatus(), m.getObjVal() if m.getNSols() else None, m.getNNodes(), round(m.getSolvingTime(), 1), flush=True)
