"""fuzz_intsing.py の 1 つの seed の問題を MPS に書き、HiGHS・SCIP・当方 (環境変数のまま) で解いて並べる。使い方: python check_seed.py seed 出力.mps"""
import sys, importlib.util
from pathlib import Path
spec = importlib.util.spec_from_file_location("fz", Path(__file__).with_name("fuzz_intsing.py"))
src = open(spec.origin).read(); src = src[:src.index("N = int(sys.argv[1])")]
ns = {"__file__": spec.origin}; exec(src, ns)
lp = ns["gen"](int(sys.argv[1])); lo, up, c, it, rows, rlo, rup = lp
import highspy
h = highspy.Highs(); h.setOptionValue("output_flag", False); K = highspy.kHighsInf; inf = float("inf")
for j in range(len(lo)):
    h.addVar(lo[j] if lo[j] > -inf else -K, up[j] if up[j] < inf else K); h.changeColCost(j, c[j])
    if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
for r, l, u in zip(rows, rlo, rup): h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
h.writeModel(sys.argv[2]); print("highs", ns["highs"](lp))
import pyscipopt
m = pyscipopt.Model(); m.hideOutput(); m.readProblem(sys.argv[2]); m.optimize(); print("scip", m.getStatus(), m.getObjVal() if m.getStatus() == "optimal" else None)
st, ob, x = ns["ours"](lp); print("ours", st, ob)
for j in range(len(lo)): print(j, "int" if it[j] else "cont", lo[j], up[j], c[j], x[j] if x else None)
for r, l, u in zip(rows, rlo, rup): print(l, u, r)
