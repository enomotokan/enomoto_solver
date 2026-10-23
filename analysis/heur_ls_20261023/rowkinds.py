"""行の形を数える: python -I rowkinds.py <dump> [行番号 ...] (行番号を渡せばその行のまわりを出す)"""
import sys, numpy as np, highspy, scipy.sparse as sp, collections
d = sys.argv[1]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f"{d}/prob.mps"); lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
A = sp.csc_matrix((lp.a_matrix_.value_, lp.a_matrix_.index_, lp.a_matrix_.start_), shape=(m, n)); R = A.tocsr()
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_); rlo, rup = np.array(lp.row_lower_), np.array(lp.row_upper_)
integ = np.array([int(t) == 1 for t in lp.integrality_])
cost = np.array(lp.col_cost_)
def ct(j):
    if lo[j] == up[j]: return "fix"
    if integ[j] and lo[j] == 0 and up[j] == 1: return "B"
    return "I" if integ[j] else "C"
def kind(i):
    s, e = R.indptr[i], R.indptr[i+1]
    t = [(ct(j), a) for j, a in zip(R.indices[s:e], R.data[s:e]) if ct(j) != "fix"]
    typ = "=" if rlo[i] == rup[i] else ("<=" if np.isinf(rlo[i]) else (">=" if np.isinf(rup[i]) else "rng"))
    c = collections.Counter(k for k, _ in t)
    co = sorted(set(a for _, a in t))
    cs = "±1" if set(co) <= {1.0, -1.0} else ("1" if co == [1.0] else f"{len(co)} vals")
    rhs = rlo[i] if typ == ">=" else rup[i]
    L = len(t)
    Lb = "1" if L == 1 else "2" if L == 2 else "3" if L == 3 else "4-10" if L <= 10 else "11-50" if L <= 50 else ">50"
    return f"{typ} len {Lb} {dict(sorted(c.items()))} coef {cs} rhs {rhs:g}"
K = collections.Counter(kind(i) for i in range(m))
for k, v in K.most_common(25): print(v, k)
print("列:", collections.Counter(ct(j) for j in range(n)), "費用の非零:", collections.Counter(ct(j) for j in range(n) if cost[j] != 0))
