"""MPS で指定の列を含む行を表示: python rows_of.py <mps> <列名> ..."""
import sys, highspy, numpy as np
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1]); lp = h.getLp()
names = list(lp.col_names_); rn = list(lp.row_names_) if len(lp.row_names_) else None
idx = {n: j for j, n in enumerate(names)}
A = lp.a_matrix_; st, ix, vl = np.array(A.start_), np.array(A.index_), np.array(A.value_)
rows = {}
want = [idx[n] for n in sys.argv[2:] if n in idx]
wr = set()
for j in want:
    for p in range(st[j], st[j + 1]): wr.add(ix[p])
for j in range(lp.num_col_):
    for p in range(st[j], st[j + 1]):
        if ix[p] in wr: rows.setdefault(ix[p], []).append((names[j], vl[p]))
for j in want:
    print(f"col {names[j]} [{lp.col_lower_[j]:g},{lp.col_upper_[j]:g}] cost {lp.col_cost_[j]:g} int {lp.integrality_[j] if len(lp.integrality_) else '-'}")
for i, r in rows.items():
    print(f"  {rn[i] if rn else i}: {lp.row_lower_[i]:g} <= " + " ".join(f"{v:+g}{n}" for n, v in r[:20]) + (" ..." if len(r) > 20 else "") + f" <= {lp.row_upper_[i]:g}")
