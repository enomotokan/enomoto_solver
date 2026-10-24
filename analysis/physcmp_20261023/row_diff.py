"""2 つの MPS (同じ元の列名) の行を (列名, 係数, 側) の集合で比べ、片方にだけある行の形を数える。python row_diff.py <a.mps> <b.mps>"""
import sys, highspy, numpy as np
from collections import Counter
def load(f):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f); lp = h.getLp()
    nm = list(lp.col_names_); A = lp.a_matrix_; st, ix, vl = np.array(A.start_), np.array(A.index_), np.array(A.value_)
    rows = [[] for _ in range(lp.num_row_)]
    for j in range(lp.num_col_):
        for p in range(st[j], st[j + 1]): rows[ix[p]].append((nm[j], vl[p]))
    out = {}
    for i, r in enumerate(rows):
        r.sort()
        key = tuple(nm for nm, _ in r)
        out.setdefault(key, []).append((tuple(v for _, v in r), lp.row_lower_[i], lp.row_upper_[i]))
    integ = {nm[j]: (lp.integrality_[j] != highspy.HighsVarType.kContinuous) for j in range(lp.num_col_)}
    return out, set(nm), integ
A, ca, ia = load(sys.argv[1]); B, cb, ib = load(sys.argv[2])
print("cols only in a", len(ca - cb), "only in b", len(cb - ca))
onlyA = [k for k in A if k not in B]; onlyB = [k for k in B if k not in A]
def shape(k, D, integ):
    c, lo, up = D[k][0]
    unit = all(abs(abs(v) - 1) < 1e-9 for v in c)
    return (len(k) if len(k) < 6 else "6+", "unit" if unit else "gen", "eq" if lo == up else "ineq", "int" if all(integ.get(n, False) for n in k) else "mixed")
print("row sets only in a", len(onlyA), Counter(shape(k, A, ia) for k in onlyA).most_common(8))
print("row sets only in b", len(onlyB), Counter(shape(k, B, ib) for k in onlyB).most_common(8))
for k in onlyA[:8]:
    c, lo, up = A[k][0]; print("  a:", lo, " ".join(f"{v:+g}{n}" for n, v in zip(k, c)), up)
