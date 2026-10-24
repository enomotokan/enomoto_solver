"""HiGHS の前処理で係数が変わった行のうち、列の組が元と同じ行 (係数の強化だけ) を元の問題に写して LP 緩和を測る:
python hcoef_apply.py <mps> [キー=値 ...]。固定 (hdiff の tsv、第 2 引数が .tsv なら) も合わせた値も出す。"""
import sys
import numpy as np
import highspy
from scipy.sparse import csc_matrix, lil_matrix
mps = sys.argv[1]; args = sys.argv[2:]
fixtsv = args.pop(0) if args and args[0].endswith(".tsv") else None
h = highspy.Highs(); h.setOptionValue("output_flag", False)
for kv in args:
    k, v = kv.split("="); h.setOptionValue(k, int(v))
h.readModel(mps)
def mat(lp):
    a = lp.a_matrix_
    return csc_matrix((np.array(a.value_), np.array(a.index_), np.array(a.start_)), shape=(lp.num_row_, lp.num_col_)).tocsr()
l0 = h.getLp(); A0 = mat(l0)
h.presolve(); l1 = h.getPresolvedLp(); A1 = mat(l1)
rn0 = {n: i for i, n in enumerate(l0.row_names_)}; cn0 = {n: j for j, n in enumerate(l0.col_names_)}
c1 = list(l1.col_names_)
A = A0.tolil(); rlo = np.array(l0.row_lower_); rup = np.array(l0.row_upper_)
cnt = 0
for i, n in enumerate(l1.row_names_):
    i0 = rn0.get(n)
    if i0 is None: continue
    s0 = set(A0[i0].indices.tolist()); r1 = {cn0[c1[j]]: v for j, v in zip(A1[i].indices, A1[i].data)}
    if set(r1) == s0 and any(abs(A0[i0, j] - v) > 1e-9 for j, v in r1.items()):
        # 下限が 0 でない列がない行だけ (ずらしの補正をしない)
        if any(l0.col_lower_[j] != 0 for j in r1): continue
        for j, v in r1.items(): A[i0, j] = v
        rlo[i0], rup[i0] = l1.row_lower_[i], l1.row_upper_[i]; cnt += 1
lo = np.array(l0.col_lower_); up = np.array(l0.col_upper_)
def lpval(lo, up):
    q = highspy.Highs(); q.setOptionValue("output_flag", False); q.readModel(mps)
    lp = q.getLp(); lp.integrality_ = []
    Ac = A.tocsc(); lp.a_matrix_.start_ = Ac.indptr; lp.a_matrix_.index_ = Ac.indices; lp.a_matrix_.value_ = Ac.data
    lp.row_lower_ = rlo; lp.row_upper_ = rup; lp.col_lower_ = lo; lp.col_upper_ = up
    q.passModel(lp); q.run(); return q.getInfo().objective_function_value
print("rows copied", cnt, "LP", lpval(lo, up))
if fixtsv:
    for line in open(fixtsv):
        nm, kind, v, _ = line.split("\t")
        if kind == "fix": j = cn0[nm]; lo[j] = up[j] = round(float(v))
    print("with fixes LP", lpval(lo, up))
