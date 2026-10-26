"""HiGHS の前処理で係数が変わった行を出す: python hcoef.py <mps> [キー=値 ...]
両方に残る列の係数が違う行 (係数の強化・置換による変化) を列ごとに出す。行の中身 (元と後) も出す。"""
import sys
import numpy as np
import highspy
from scipy.sparse import csc_matrix
mps = sys.argv[1]
h = highspy.Highs(); h.setOptionValue("output_flag", False)
for kv in sys.argv[2:]:
    k, v = kv.split("="); h.setOptionValue(k, int(v))
h.readModel(mps)
def mat(lp):
    a = lp.a_matrix_
    return csc_matrix((np.array(a.value_), np.array(a.index_), np.array(a.start_)), shape=(lp.num_row_, lp.num_col_)).tocsr()
l0 = h.getLp(); A0 = mat(l0)
h.presolve(); l1 = h.getPresolvedLp(); A1 = mat(l1)
rn0 = {n: i for i, n in enumerate(l0.row_names_)}
c0 = list(l0.col_names_); c1 = list(l1.col_names_); kept = set(c1)
nchg = 0
for i, n in enumerate(l1.row_names_):
    i0 = rn0.get(n)
    if i0 is None: continue
    r0 = {c0[j]: v for j, v in zip(A0[i0].indices, A0[i0].data)}
    r1 = {c1[j]: v for j, v in zip(A1[i].indices, A1[i].data)}
    common = [c for c in r1 if c in r0]
    diff = [c for c in common if abs(r0[c] - r1[c]) > 1e-9]
    added = [c for c in r1 if c not in r0]
    if diff:
        nchg += 1
        if nchg <= 40:
            print(f"{n}: orig [{l0.row_lower_[i0]:g},{l0.row_upper_[i0]:g}] {sorted(r0.items())} -> [{l1.row_lower_[i]:g},{l1.row_upper_[i]:g}] {sorted(r1.items())}")
print("rows with changed coefficients", nchg)
