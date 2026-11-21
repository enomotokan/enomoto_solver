"""デバッグ解 (元の列の順) の元の問題での行・境界・整数性の違反を出す。python solviol.py MPS SOL"""
import sys
import numpy as np, highspy
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1])
lp = h.getLp(); x = np.array([float(l) for l in open(sys.argv[2]) if l.strip()])
a = lp.a_matrix_; n = lp.num_col_; m = lp.num_row_
act = np.zeros(m); scale = np.zeros(m)
for j in range(n):
    for k in range(a.start_[j], a.start_[j + 1]):
        act[a.index_[k]] += a.value_[k] * x[j]; scale[a.index_[k]] += abs(a.value_[k] * x[j])
lo, up = np.array(lp.row_lower_), np.array(lp.row_upper_)
v = np.maximum(lo - act, act - up); v[~np.isfinite(v)] = 0
o = np.argsort(-v)[:10]
print("max row violation", v.max(), "rows > 1e-9:", int((v > 1e-9).sum()), "rows > 1e-7:", int((v > 1e-7).sum()))
for i in o:
    if v[i] > 0: print(" row", i, lp.row_names_[i] if lp.row_names_ else "", "act", act[i], "[", lo[i], up[i], "] viol", v[i])
cl, cu = np.array(lp.col_lower_), np.array(lp.col_upper_)
print("max bound violation", max(0, (cl - x).max(), (x - cu).max()))
integ = [j for j, t in enumerate(lp.integrality_) if int(t) == 1] if lp.integrality_ else []
print("max integrality violation", max([abs(x[j] - round(x[j])) for j in integ] or [0]), "n int", len(integ))
print("obj", float(np.dot(lp.col_cost_, x)) + lp.offset_)
