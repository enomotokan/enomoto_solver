"""fuzz_symdyn.py の 1 問を MPS に書き出す: python symfuzz_mps.py 種 大きさの倍率 出力.mps (HiGHS で解いた最適値も表示)"""
import os, sys
seed, sc, out = int(sys.argv[1]), sys.argv[2], sys.argv[3]
sys.argv = ["fuzz_symdyn.py", "1", str(seed), sc]
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "symdyn_20261110"))
import highspy  # noqa: E402
import fuzz_symdyn as fz  # noqa: E402
p, kind = fz.gen(seed)
h = highspy.Highs(); h.setOptionValue('output_flag', False); h.setOptionValue('mip_rel_gap', 1e-9)
K = highspy.kHighsInf
for j in range(len(p.lo)):
    h.addVar(p.lo[j] if p.lo[j] > -fz.inf else -K, p.up[j] if p.up[j] < fz.inf else K); h.changeColCost(j, p.c[j])
    if p.it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
for r, l, u in zip(p.rows, p.rlo, p.rup):
    h.addRow(l if l > -K else -K, u if u < K else K, len(r), [a for a, _ in r], [b for _, b in r])
h.writeModel(out); h.run()
print(kind, "cols", len(p.lo), "rows", len(p.rows), "HiGHS", h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value)
