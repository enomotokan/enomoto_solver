"""書き出した LP の出発の基底 B (構造列 + スラック) の条件数を見積もる (scipy の splu と onenormest)。
使い方: python cond_basis.py <prefix> [scale]  (scale を付けると HiGHS と同じく尺度変換した LP の基底でも見る)"""
import sys
import numpy as np, scipy.sparse as sp, scipy.sparse.linalg as sla
import highspy
prefix = sys.argv[1]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(prefix + ".mps")
lp = h.getLp(); n, m = lp.num_col_, lp.num_row_
A = sp.csc_matrix((np.array(lp.a_matrix_.value_), np.array(lp.a_matrix_.index_), np.array(lp.a_matrix_.start_)), shape=(m, n))
b = [int(t) for t in open(prefix + ".bas").read().split("\n")[1].split()]
# ENOMOTO のスラックは +I (行 i のスラック n+i)
cols = [A[:, j] if j < n else sp.csc_matrix(([1.0], ([j - n], [0])), shape=(m, 1)) for j in b]
B = sp.hstack(cols).tocsc()
lu = sla.splu(B)
Binv = sla.LinearOperator((m, m), matvec=lu.solve, rmatvec=lambda x: lu.solve(x, trans="T"), dtype=float)
nB = sla.norm(B, 1); nBi = sla.onenormest(Binv)
print(f"{prefix}: m={m} structural basic={sum(1 for j in b if j < n)} |B|_1={nB:.3e} |B^-1|_1~{nBi:.3e} cond_1~{nB*nBi:.3e}")
# 行と列を 1 ノルムで均した尺度 (簡易な平衡化) でも見る
r = 1.0 / np.maximum(abs(B).max(axis=1).toarray().ravel(), 1e-300)
Bs = sp.diags(r) @ B
c = 1.0 / np.maximum(abs(Bs).max(axis=0).toarray().ravel(), 1e-300)
Bs = (Bs @ sp.diags(c)).tocsc()
lu2 = sla.splu(Bs)
Bi2 = sla.LinearOperator((m, m), matvec=lu2.solve, rmatvec=lambda x: lu2.solve(x, trans="T"), dtype=float)
print(f"  equilibrated: cond_1~{sla.norm(Bs,1)*sla.onenormest(Bi2):.3e}")
