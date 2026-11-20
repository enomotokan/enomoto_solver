"""P2-INF: SCIP で 1 問を解き、統計 (分枝・衝突・分離・伝播の表) を書き出す。python scip_stats.py <mps> <秒> <出力>"""
import sys
from pyscipopt import Model
m = Model()
m.readProblem(sys.argv[1])
m.setParam("limits/time", float(sys.argv[2]))
m.setParam("display/verblevel", 4)
m.optimize()
m.writeStatistics(sys.argv[3])
print(m.getStatus(), m.getObjVal() if m.getNSols() else None, m.getNNodes(), m.getSolvingTime())
