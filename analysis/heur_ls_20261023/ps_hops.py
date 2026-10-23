"""FJ の書き出した点 (ENOMOTO_MIP_FJ_DUMP) の違反行から、行-列のグラフで何段で整数列がどれだけ入るかを数える。
python -I ps_hops.py <dumpディレクトリ> <点のファイル>"""
import sys, numpy as np, highspy, scipy.sparse as sp
d, ptf = sys.argv[1], sys.argv[2]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f"{d}/prob.mps"); lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
A = sp.csc_matrix((lp.a_matrix_.value_, lp.a_matrix_.index_, lp.a_matrix_.start_), shape=(m, n)); R = A.tocsr()
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_); rlo, rup = np.array(lp.row_lower_), np.array(lp.row_upper_)
integ = np.array([int(t) == 1 for t in lp.integrality_])
x = np.array([float(l) for l in open(ptf)]); act = R @ x
V = [i for i in range(m) if act[i] > rup[i] + 1e-6 or act[i] < rlo[i] - 1e-6]
free = (lo < up)
dist = {}; fr = set(V); seen = set(V)
for lvl in range(6):
    cols = set()
    for i in fr:
        for j in R.indices[R.indptr[i]:R.indptr[i+1]]:
            if j not in dist: dist[j] = lvl; cols.add(j)
    nr = set()
    for j in cols:
        for i in A.indices[A.indptr[j]:A.indptr[j+1]]:
            if i not in seen: seen.add(i); nr.add(i)
    fr = nr
    tot = [j for j in dist if free[j] and integ[j]]
    print(lvl, "int free cols cumul", len(tot), "rows", len(seen))
print("int free total", (free & integ).sum())
