import sys, highspy, numpy as np
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1])
lp = h.getLp(); A = lp.a_matrix_
st, ix, va = np.array(A.start_), np.array(A.index_), np.array(A.value_)
# colwise
cols = [int(c) for c in sys.argv[2:]]
rows = set()
for c in cols:
    for k in range(st[c], st[c+1]): rows.add(int(ix[k]))
# build rowwise
import scipy.sparse as sp
M = sp.csc_matrix((va, ix, st), shape=(lp.num_row_, lp.num_col_)).tocsr()
for r in sorted(rows):
    row = M.getrow(r)
    terms = " ".join(f"{v:+g}*C{j}[{lp.col_lower_[j]:g},{lp.col_upper_[j]:g}{'I' if int(lp.integrality_[j]) else ''}]" for j, v in zip(row.indices, row.data))
    print(f"R{r}: {lp.row_lower_[r]:g} <= {terms[:600]} <= {lp.row_upper_[r]:g}")
for c in cols: print("C",c, lp.col_lower_[c], lp.col_upper_[c], lp.col_cost_[c])
