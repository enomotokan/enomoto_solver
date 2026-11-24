"""HiGHS・SCIP で neos-5188808-nattai を 1 スレッド 600 秒で解き、返す解を x_<名>.json に書き出す (check.py・fixlp.py で確かめる)。
使い方: python others.py highs|scip [秒]"""
import json, sys, time
from pathlib import Path
R = Path(__file__).resolve().parents[2]; MPS = str(R / ".miplib_cache/neos-5188808-nattai.mps")
who = sys.argv[1]; tl = float(sys.argv[2]) if len(sys.argv) > 2 else 600.0
t = time.perf_counter()
if who == "highs":
    import highspy
    h = highspy.Highs(); h.setOptionValue("time_limit", tl); h.setOptionValue("threads", 1); h.readModel(MPS); h.run()
    lp = h.getLp(); info = h.getInfo()
    x = dict(zip(lp.col_names_, h.getSolution().col_value))
    out = {"status": h.modelStatusToString(h.getModelStatus()), "objective": info.objective_function_value, "bound": info.mip_dual_bound,
           "max_viol": [info.max_primal_infeasibility, info.max_integrality_violation], "x": x}
else:
    import pyscipopt
    m = pyscipopt.Model(); m.hideOutput(); m.setParam("limits/time", tl); m.setParam("parallel/maxnthreads", 1); m.readProblem(MPS); m.optimize()
    s = m.getBestSol(); x = {v.name: m.getSolVal(s, v) for v in m.getVars()}
    out = {"status": m.getStatus(), "objective": m.getPrimalbound(), "bound": m.getDualbound(), "x": x}
out["time"] = time.perf_counter() - t
Path(f"x_{who}.json").write_text(json.dumps(out))
print(json.dumps({k: v for k, v in out.items() if k != "x"}))
