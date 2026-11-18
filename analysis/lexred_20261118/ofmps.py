"""fuzz_of.py の 1 問を MPS に書き出す: python ofmps.py 種 大きさ 出力.mps (HiGHS の最適値も表示)"""
import os, sys
seed, sc, out = int(sys.argv[1]), sys.argv[2], sys.argv[3]
sys.argv = ["fuzz_of.py", "1", str(seed), sc]
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "sym_of_20261028"))
import highspy  # noqa: E402
import fuzz_of as fz  # noqa: E402
lo, up, c, rows, rlo, rup, kind, n = fz.gen(seed)
h = highspy.Highs(); h.setOptionValue('output_flag', False); h.setOptionValue('mip_rel_gap', 0.0)
K = highspy.kHighsInf
for j in range(len(lo)):
    h.addVar(lo[j], up[j]); h.changeColCost(j, c[j]); h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
for r, l, u in zip(rows, rlo, rup):
    h.addRow(max(l, -K), min(u, K), len(r), [j for j, _ in r], [a for _, a in r])
h.writeModel(out); h.run()
print(kind, "n", n, "cols", len(lo), "rows", len(rows), "HiGHS", h.getInfo().objective_function_value)
