"""HiGHS の前処理で列が足された集合詰め込み行 (元が 0-1 列の係数 1・上限 1) を元の問題に写して (消えた列がある行と係数 -1 の文字は除く) LP 緩和を測る:
python hclq_apply.py <mps> [キー=値 ...]。新しい行 = 元の行 ∪ 足された列 (係数は前処理後のもの、-1 の文字は右辺を 1 減らす)。
写した問題を mps/<元>_hclq.mps に書く。"""
import sys, os
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
rn0 = {n: i for i, n in enumerate(l0.row_names_)}; cn0 = {n: j for j, n in enumerate(l0.col_names_)}; c1 = list(l1.col_names_)
isbin = lambda j: l0.col_lower_[j] == 0 and l0.col_upper_[j] == 1
A = A0.tolil(); rlo = np.array(l0.row_lower_); rup = np.array(l0.row_upper_)
nrow = nadd = skipped = 0
for i, n in enumerate(l1.row_names_):
    i0 = rn0.get(n)
    if i0 is None: continue
    r0 = dict(zip(A0[i0].indices.tolist(), A0[i0].data.tolist()))
    if not (rup[i0] == 1 and not np.isfinite(rlo[i0]) and all(v == 1 and isbin(j) for j, v in r0.items())): continue
    r1 = {cn0[c1[j]]: v for j, v in zip(A1[i].indices, A1[i].data)}
    add = {j: v for j, v in r1.items() if j not in r0}
    # 置換で消えた列がある行は使わない (置換の相手が足された列に見える)。足すのは係数 +1 だけ
    if not add or any(j not in r1 for j in r0) or any(v != 1 for v in add.values()): skipped += 1; continue
    for j, v in add.items():
        A[i0, j] = v
        if v < 0: rup[i0] -= 1
    nrow += 1; nadd += len(add)
h0 = highspy.Highs(); h0.setOptionValue("output_flag", False); h0.readModel(mps); lp = h0.getLp(); Ac = A.tocsc()
lp.a_matrix_.start_ = Ac.indptr; lp.a_matrix_.index_ = Ac.indices; lp.a_matrix_.value_ = Ac.data; lp.row_lower_ = rlo; lp.row_upper_ = rup
g = highspy.Highs(); g.setOptionValue("output_flag", False); g.passModel(lp)
out = "mps/" + os.path.basename(mps).replace(".mps", "_hclq.mps"); g.writeModel(out)
lp.integrality_ = []; q = highspy.Highs(); q.setOptionValue("output_flag", False); q.passModel(lp); q.run()
print("rows extended", nrow, "skipped", skipped, "nonzeros added", nadd, "LP", q.getModelStatus(), q.getInfo().objective_function_value, "->", out)
