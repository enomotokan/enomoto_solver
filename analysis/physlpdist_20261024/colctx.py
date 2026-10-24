"""列の周り: python colctx.py <接頭辞k> <列名> [解のファイル]。列が入る行と、その行の各列の LP 値・解の値・境界。"""
import sys, numpy as np, highspy, scipy.sparse as sp
pre, name = sys.argv[1], sys.argv[2]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(pre + ".mps"); lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
A = lp.a_matrix_; M = sp.csc_matrix((A.value_, A.index_, A.start_), shape=(m, n)); R = M.tocsr()
x = np.loadtxt(pre + "_x.txt"); xs = np.loadtxt(sys.argv[3]) if len(sys.argv) > 3 else None
names = list(lp.col_names_); j = names.index(name)
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
print(f"{name}: LP {x[j]:.4f}, bounds [{lo[j]}, {up[j]}], cost {lp.col_cost_[j]}")
for i in M[:, j].nonzero()[0]:
    row = R[i]
    terms = " + ".join(f"{v:g}*{names[k]}(lp {x[k]:.3f}{'' if xs is None else f', sol {xs[k]:.0f}'}{'' if lo[k] < up[k] else ' fixed'})" for k, v in zip(row.indices, row.data))
    print(f"  row {lp.row_names_[i] if lp.row_names_ else i}: {lp.row_lower_[i]:g} <= {terms} <= {lp.row_upper_[i]:g}")
