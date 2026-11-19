"""SCIP (pyscipopt) で 1 問を解き、統計 (前処理の各段の削った列・行、分離の各段のカット、分枝など) を書き出す。
python scip_stats.py <問題> <種> <秒> [設定=値 ...]  → logs/<問題>_scipstat_s<種>.txt"""
import sys
from pathlib import Path
from pyscipopt import Model
D = Path(__file__).resolve().parent; WT = D.parents[1]
p, seed, tl = sys.argv[1], int(sys.argv[2]), float(sys.argv[3])
m = Model(); m.hideOutput(); m.readProblem(str(WT / ".miplib_cache" / f"{p}.mps"))
m.setParam("limits/time", tl); m.setParam("randomization/randomseedshift", seed)
for kv in sys.argv[4:]:
    k, v = kv.split("=", 1)
    try: v = int(v)
    except ValueError:
        try: v = float(v)
        except ValueError: v = {"true": True, "false": False}.get(v, v)
    m.setParam(k, v)
m.optimize()
out = D / "logs" / f"{p}_scipstat_s{seed}{'_' + '_'.join(a.split('=')[0].split('/')[-1] for a in sys.argv[4:]) if len(sys.argv) > 4 else ''}.txt"
m.writeStatistics(str(out))
print(p, m.getStatus(), m.getObjVal() if m.getNSols() else None, m.getSolvingTime(), m.getNNodes(), out)
