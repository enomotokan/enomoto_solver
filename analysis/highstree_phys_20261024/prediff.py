"""ENOMOTO の前処理の後の 2 つの書き出し (探りあり A・なし B) の違いを見る: python -I prediff.py <A.mps> <B.mps>
列名 (C<元の番号>) で突き合わせ、列・行の数、境界の違い、行の形 (長さ・等式・係数) の違いを数える"""
import sys
from collections import Counter
import highspy, numpy as np
def load(f):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f)
    lp = h.getLp(); n, m = lp.num_col_, lp.num_row_
    names = [h.getColName(j)[1] for j in range(n)]
    A = lp.a_matrix_; st, idx, val = np.array(A.start_), np.array(A.index_), np.array(A.value_)
    rows = [[] for _ in range(m)]
    for j in range(n):
        for k in range(st[j], st[j+1]):
            rows[idx[k]].append((names[j], round(float(val[k]), 9)))
    rl, ru = np.array(lp.row_lower_), np.array(lp.row_upper_)
    rset = Counter((tuple(sorted(r)), round(float(rl[i]), 9), round(float(ru[i]), 9)) for i, r in enumerate(rows) if r)
    cl, cu = np.array(lp.col_lower_), np.array(lp.col_upper_)
    integ = [int(t) for t in lp.integrality_]
    cols = {names[j]: (float(cl[j]), float(cu[j]), integ[j], st[j+1] - st[j]) for j in range(n)}
    return cols, rset, rows, rl, ru
ca, ra, rowsa, rla, rua = load(sys.argv[1]); cb, rb, rowsb, rlb, rub = load(sys.argv[2])
free = lambda c: {k for k, v in c.items() if v[0] < v[1] and v[3] > 0}
fa, fb = free(ca), free(cb)
print("自由な列 A", len(fa), "B", len(fb), "A だけ", len(fa - fb), "B だけ", len(fb - fa))
print("B だけで自由な列の A での状態", Counter(("固定" if ca[k][0] == ca[k][1] else "行なし" if ca[k][3] == 0 else "?") if k in ca else "なし" for k in fb - fa).most_common(4))
print("B だけで自由な列の種類 (整数か)", Counter(cb[k][2] for k in fb - fa))
bd = [(k, ca[k][:2], cb[k][:2]) for k in fa & fb if ca[k][:2] != cb[k][:2]]
print("両方で自由で境界が違う列", len(bd), bd[:4])
print("行 (形で数えた) A", sum(ra.values()), "B", sum(rb.values()), "A だけの形", sum((ra - rb).values()), "B だけの形", sum((rb - ra).values()))
la = Counter(len(r[0]) for r in (ra - rb).elements()); lb = Counter(len(r[0]) for r in (rb - ra).elements())
print("A だけの行の長さ", sorted(la.items())[:10]); print("B だけの行の長さ", sorted(lb.items())[:10])
ex = list((rb - ra).elements())[:3]
for r in ex:
    print("B だけの行の例", r[0][:6], r[1], r[2])
ex = list((ra - rb).elements())[:3]
for r in ex:
    print("A だけの行の例", r[0][:6], r[1], r[2])
