"""同じ名前の行・列を持つ 2 つの MPS の行の違い (HiGHS の規則が何を書き換えたか): rowdiff.py <a.mps> <b.mps> [見本の数]"""
import sys
from collections import Counter
import highspy, numpy as np

def rd(p):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(p); lp = h.getLp()
    cn, rn = list(lp.col_names_), list(lp.row_names_)
    a = lp.a_matrix_; st, ix, vl = a.start_, a.index_, a.value_
    rows = {r: {} for r in rn}
    for j in range(lp.num_col_):
        for k in range(st[j], st[j + 1]):
            rows[rn[ix[k]]][cn[j]] = vl[k]
    integ = list(lp.integrality_)
    isint = {cn[j]: (len(integ) > 0 and integ[j] == highspy.HighsVarType.kInteger) for j in range(lp.num_col_)}
    bnd = {rn[i]: (lp.row_lower_[i], lp.row_upper_[i]) for i in range(lp.num_row_)}
    return rows, bnd, isint, set(cn)

A, Ab, Ai, Ac = rd(sys.argv[1]); B, Bb, Bi, Bc = rd(sys.argv[2])
k = int(sys.argv[3]) if len(sys.argv) > 3 else 8
print("rows only in a", len(set(A) - set(B)), "only in b", len(set(B) - set(A)), "cols only in a", len(Ac - Bc))
ch = [r for r in A if r in B and (A[r] != B[r] or Ab[r] != Bb[r])]
print("changed rows", len(ch))
c = Counter()
for r in ch:
    c[(len(A[r]), len(B[r]), Ab[r][0] == Ab[r][1])] += 1
print("(len a, len b, eq):", c.most_common(10))
for r in ch[:k]:
    print(r, Ab[r], {x: round(v, 3) for x, v in A[r].items()}, "->", Bb[r], {x: round(v, 3) for x, v in B[r].items()})
