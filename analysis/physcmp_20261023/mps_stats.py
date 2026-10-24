"""前処理後の MPS の形の統計: python mps_stats.py <mps>"""
import sys
from collections import defaultdict
import highspy, numpy as np
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1])
lp = h.getLp(); m, n = lp.num_row_, lp.num_col_
a = lp.a_matrix_; start, idx, val = np.array(a.start_), np.array(a.index_), np.array(a.value_)
rows = defaultdict(list)
for j in range(n):
    for k in range(start[j], start[j+1]):
        rows[idx[k]].append((j, val[k]))
lo, up = np.array(lp.row_lower_), np.array(lp.row_upper_)
cl, cu = np.array(lp.col_lower_), np.array(lp.col_upper_)
integ = np.array([int(t) for t in lp.integrality_]) if len(lp.integrality_) else np.zeros(n)
free = (cl < cu)
eq = sum(1 for i in range(m) if lo[i] == up[i])
eq2 = sum(1 for i in range(m) if lo[i] == up[i] and len(rows[i]) == 2)
le2 = sum(1 for i in range(m) if lo[i] != up[i] and len(rows[i]) == 2)
nnz = sum(len(r) for r in rows.values())
lens = np.array([len(rows[i]) for i in range(m)])
print(f"{sys.argv[1]}: rows {m} cols {n} (free {free.sum()}, int {int((integ>0).sum())}) nnz {nnz}; eq {eq} (2-var eq {eq2}); 2-var ineq {le2}; row len mean {lens.mean():.2f} max {lens.max()} >10: {(lens>10).sum()}")
