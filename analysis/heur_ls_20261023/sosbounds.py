"""30n20b8: 定義の等式 (sum t y_t - s = 0、y は sum y = 1 の 0-1) から s の境界 [min t, max t] (自由な y の t) を出し、
precedence (2 列の行) で伝播したとき、どれだけ境界が締まるかと、FJ の点がその境界を破っているか。python -I sosbounds.py <dump> <点>"""
import sys, numpy as np, highspy, scipy.sparse as sp
d = sys.argv[1]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f"{d}/prob.mps"); lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
A = sp.csc_matrix((lp.a_matrix_.value_, lp.a_matrix_.index_, lp.a_matrix_.start_), shape=(m, n)); R = A.tocsr()
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_); rlo, rup = np.array(lp.row_lower_), np.array(lp.row_upper_)
integ = np.array([int(t) == 1 for t in lp.integrality_])
x = np.array([float(l) for l in open(sys.argv[2])])
binc = integ & (lo == 0) & (up == 1)
row = lambda i: list(zip(R.indices[R.indptr[i]:R.indptr[i+1]], R.data[R.indptr[i]:R.indptr[i+1]]))
# 集合分割の行 (0-1 だけ、係数 1、= 1、固定 0 の列は除く)
sp_rows = [set(j for j, a in row(i) if binc[j]) for i in range(m) if rlo[i] == rup[i] == 1 and all(a == 1 and (binc[j] or (lo[j] == up[j] == 0)) for j, a in row(i))]
L, U = lo.copy(), up.copy()
for i in range(m):
    if rlo[i] != rup[i]: continue
    t = row(i)
    ints = [(j, a) for j, a in t if not binc[j] and lo[j] < up[j]]
    if len(ints) != 1 or abs(ints[0][1]) != 1: continue
    bs = set(j for j, a in t if binc[j])
    if not any(bs <= s for s in sp_rows): continue
    j0, a0 = ints[0]
    vals = [-a / a0 for j, a in t if binc[j]]
    k = rlo[i] / a0
    L[j0] = max(L[j0], min(vals) + k); U[j0] = min(U[j0], max(vals) + k)
ch0 = int(((L > lo) | (U < up)).sum())
# precedence などの 2 列の行で伝播
for _ in range(100):
    chg = False
    for i in range(m):
        t = row(i)
        if len(t) != 2: continue
        (j1, a1), (j2, a2) = t
        for (p, ap), (q, aq) in [((j1, a1), (j2, a2)), ((j2, a2), (j1, a1))]:
            # ap x_p + aq x_q <= rup  => ap x_p <= rup - min(aq x_q)
            if np.isfinite(rup[i]):
                mn = aq * (L[q] if aq > 0 else U[q])
                b = (rup[i] - mn) / ap
                if ap > 0 and b < U[p] - 1e-9: U[p] = np.floor(b + 1e-9) if integ[p] else b; chg = True
                if ap < 0 and b > L[p] + 1e-9: L[p] = np.ceil(b - 1e-9) if integ[p] else b; chg = True
            if np.isfinite(rlo[i]):
                mx = aq * (U[q] if aq > 0 else L[q])
                b = (rlo[i] - mx) / ap
                if ap > 0 and b > L[p] + 1e-9: L[p] = np.ceil(b - 1e-9) if integ[p] else b; chg = True
                if ap < 0 and b < U[p] - 1e-9: U[p] = np.floor(b + 1e-9) if integ[p] else b; chg = True
    if not chg: break
tight = np.where((L > lo + 1e-9) | (U < up - 1e-9))[0]
print(f"集合分割の行 {len(sp_rows)}、定義の等式で締まる列 {ch0}、伝播の後に締まる列 {len(tight)}、空になる列 {(L > U + 1e-9).sum()}")
bad = [j for j in tight if x[j] < L[j] - 1e-9 or x[j] > U[j] + 1e-9]
print(f"FJ の点が締めた境界を破る列 {len(bad)}: " + ", ".join(f"c{j} {x[j]:g} [{L[j]:g},{U[j]:g}] (元 [{lo[j]:g},{up[j]:g}])" for j in bad[:12]))
