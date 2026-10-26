"""プロービングの入力 (probe_N.mps) と同値の組 (probe_N.pairs) を HiGHS で確かめる。
使い方: python check_pairs.py PREFIX [TL] [SOL]
  1. 組なしの MIP を HiGHS で解き (SOL があればそれを読む)、最適解で成り立たない組を挙げる
  2. 組を全部足した LP と MIP の値を、組なしと比べる"""
import sys, highspy, numpy as np
pre = sys.argv[1]; tl = float(sys.argv[2]) if len(sys.argv) > 2 else 600
def load(pairs=None, relax=False):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("time_limit", tl)
    h.setOptionValue("mip_rel_gap", 1e-9)
    h.readModel(pre + ".mps")
    if relax:
        lp = h.getLp(); n = lp.num_col_
        h.changeColsIntegrality(n, np.arange(n, dtype=np.int32), np.zeros(n, dtype=np.uint8))
    if pairs:
        for j, k, neg in pairs:
            ij = h.getColByName(f"C{j}")[1]; ik = h.getColByName(f"C{k}")[1]
            h.addRow(float(neg), float(neg), 2, np.array([ij, ik], dtype=np.int32), np.array([1.0, 1.0 if neg else -1.0]))
    return h
pairs = [tuple(map(int, l.split())) for l in open(pre + ".pairs") if l.strip()]
print("pairs", len(pairs))
for rel in (True, False):
    for P in (None, pairs):
        h = load(P, rel); h.run()
        print("LP" if rel else "MIP", "with pairs" if P else "no pairs", h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value, "bound", h.getInfo().mip_dual_bound if not rel else "")
        if not rel and P is None:
            h0 = h
x = np.array(h0.getSolution().col_value)
names = {h0.getColByName(f"C{j}")[1]: j for j, _, _ in pairs} 
def val(j): return x[h0.getColByName(f"C{j}")[1]]
bad = [(j, k, neg, val(j), val(k)) for j, k, neg in pairs if abs((val(j) + val(k) if neg else val(j) - val(k)) - neg) > 1e-6]
print("violated by HiGHS solution:", len(bad))
for b in bad[:50]: print("  ", b)
open(pre + ".bad", "w").write("".join(f"{j} {k} {neg} {a} {b}\n" for j, k, neg, a, b in bad))
np.savetxt(pre + ".highs_sol", x)
