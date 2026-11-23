# SCIP を 1 設定で回し、時間・ノード・最初の LP の値・根の下界を 1 行の JSON で出す (analysis/scipsym_20261120/scipab.py の写し + 最初の LP の値)
# bnatt_20261123 の写し: HEUR=off・SEP=off・PRESOL=off で部品の群をまとめて切れる。
# 使い方: python scipab.py <mps> <秒> <名前> [パラメータ=値 ...]  (ログは logs/<問題>_<名前>.log)
import sys, json, os
from pyscipopt import Model
mps, tl, name = sys.argv[1], float(sys.argv[2]), sys.argv[3]
prob = os.path.basename(mps).split('.')[0]
m = Model(); m.hideOutput()
m.setParam('limits/time', tl); m.setParam('randomization/randomseedshift', 1)
from pyscipopt import SCIP_PARAMSETTING
for kv in sys.argv[4:]:
    k, v = kv.split('=')
    if k in ('HEUR', 'SEP', 'PRESOL'):  # 部品の群をまとめて切る (HEUR=off など、この写しで足した)
        getattr(m, {'HEUR': 'setHeuristics', 'SEP': 'setSeparating', 'PRESOL': 'setPresolve'}[k])(getattr(SCIP_PARAMSETTING, v.upper()))
        continue
    cur = m.getParam(k)
    v = (v.lower() in ('1','true')) if isinstance(cur, bool) else type(cur)(v)
    m.setParam(k, v)
m.readProblem(mps)
log = f'logs/{prob}_{name}.log'
m.setLogfile(log); m.hideOutput(False)
m.optimize()
m.writeStatistics(log + '.stat')
fl = next((l.split(":")[1].split()[0] for l in open(log + ".stat") if l.strip().startswith("First LP value")), None)
r = dict(first_lp=fl, prob=prob, name=name, status=m.getStatus(), time=round(m.getSolvingTime(), 2), nodes=m.getNTotalNodes(),
         dual=m.getDualbound(), primal=m.getPrimalbound() if m.getNSols() else None, root=m.getDualboundRoot(),
         nvars_trans=m.getNVars(transformed=True) if hasattr(m,'getNVars') else None)
print(json.dumps(r)); sys.stdout.flush()
