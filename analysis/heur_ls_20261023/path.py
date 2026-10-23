"""点 x から実行可能解 y へ、値の違う列を 1 本ずつ (貪欲に違反行の数が最小になる順で) 動かしたときの違反行の数の推移。
python -I path.py <dump> <点> <解>"""
import sys, numpy as np, highspy, scipy.sparse as sp
d = sys.argv[1]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f"{d}/prob.mps"); lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
A = sp.csc_matrix((lp.a_matrix_.value_, lp.a_matrix_.index_, lp.a_matrix_.start_), shape=(m, n))
rlo, rup = np.array(lp.row_lower_), np.array(lp.row_upper_)
x = np.array([float(l) for l in open(sys.argv[2])]); y = np.array([float(l) for l in open(sys.argv[3])])
ch = [j for j in range(n) if abs(x[j] - y[j]) > 1e-6]
act = A @ x
def nv(a): return int(((a > rup + 1e-6) | (a < rlo - 1e-6)).sum())
cur = x.copy(); seq = [nv(act)]
left = set(ch)
while left:
    best = None
    for j in left:
        s, e = A.indptr[j], A.indptr[j+1]
        a2 = act.copy(); a2[A.indices[s:e]] += A.data[s:e] * (y[j] - cur[j])
        v = nv(a2)
        if best is None or v < best[0]: best = (v, j, a2)
    v, j, act = best; cur[j] = y[j]; left.remove(j); seq.append(v)
print(f"列 {len(ch)} 本、違反行の数の推移 (貪欲): {seq}、途中の最大 {max(seq)}")
