"""SCIP の根だけ (limits/nodes=1) を、打ち切り値 (最適値 + 1e-6·max(1,|opt|)) つき/なしで、パラメータを変えて回す:
python scip_root.py PROB SEED TAG [cut] [param=value ...]  結果を scip_root.jsonl に足す (根の下界・run の数・前処理後の列数)"""
import json, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
from one import solu  # noqa: E402


def conv(v):
    if v in ("TRUE", "FALSE"): return v == "TRUE"
    for f in (int, float):
        try: return f(v)
        except ValueError: pass
    return v


prob, seed, tag, args = sys.argv[1], int(sys.argv[2]), sys.argv[3], sys.argv[4:]
import run_miplib_benchmark as rb  # noqa: E402
import pyscipopt  # noqa: E402
mps = rb.fetch(prob, WT / ".miplib_cache")
m = pyscipopt.Model(); m.hideOutput()
m.setParam("limits/time", 600); m.setParam("limits/nodes", 1); m.setParam("randomization/randomseedshift", seed)
m.readProblem(str(mps))
opt = solu()[prob]
for a in args:
    if a == "cut": m.setObjlimit(opt + 1e-6 * max(1.0, abs(opt)))
    else:
        k, v = a.split("=", 1); m.setParam(k, conv(v))
t = time.perf_counter(); m.optimize(); t = time.perf_counter() - t
row = {"problem": prob, "seed": seed, "tag": tag, "args": args, "root": m.getDualboundRoot(), "runs": m.getNRuns(),
       "vars": m.getNVars(True) if hasattr(m, "getNVars") else None, "time": round(t, 2), "nsols": m.getNSols()}
try: row["presolved_vars"] = m.getNVars(True)
except Exception: pass  # noqa: BLE001
with open(D / "scip_root.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
print(json.dumps(row))
