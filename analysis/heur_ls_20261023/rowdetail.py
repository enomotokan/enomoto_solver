"""行と列のまわりを詳しく出す: python -I rowdetail.py <dump> <点> <行番号> [深さ]"""
import sys, numpy as np, highspy, scipy.sparse as sp
d, ptf, r0 = sys.argv[1], sys.argv[2], int(sys.argv[3])
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f"{d}/prob.mps"); lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
A = sp.csc_matrix((lp.a_matrix_.value_, lp.a_matrix_.index_, lp.a_matrix_.start_), shape=(m, n)); R = A.tocsr()
ridx = [int(s[1:]) for s in lp.row_names_]; rpos = {r: i for i, r in enumerate(ridx)}
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_); rlo, rup = np.array(lp.row_lower_), np.array(lp.row_upper_)
cost = np.array(lp.col_cost_)
x = np.array([float(l) for l in open(ptf)]); act = R @ x
def row(i):
    s, e = R.indptr[i], R.indptr[i+1]; return list(zip(R.indices[s:e], R.data[s:e]))
def col(j):
    s, e = A.indptr[j], A.indptr[j+1]; return list(zip(A.indices[s:e], A.data[s:e]))
def rs(i):
    t = row(i)
    nz = [(j, a) for j, a in t if x[j] != 0]
    return f"r{ridx[i]} [{rlo[i]:g},{rup[i]:g}] act {act[i]:g} len {len(t)} nonzero-terms {[(f'c{j}', a, x[j]) for j, a in nz][:8]} coefs {sorted(set(a for _, a in t))[:6]}..{sorted(set(a for _, a in t))[-3:]}"
i = rpos[r0]
print("行:", rs(i))
for j, a in row(i):
    if x[j] != 0 or not (lo[j] == 0 and up[j] == 1):
        print(f" 列 c{j} a={a:g} x={x[j]:g} [{lo[j]:g},{up[j]:g}] cost {cost[j]:g}")
        for r, b in col(j):
            if r != i: print(f"    {b:g} in", rs(r))
