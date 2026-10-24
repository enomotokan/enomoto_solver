"""HiGHS の前処理後の問題で暗に整数 (kImplicitInteger) の列の形を見る: python -I implint_highs.py <mps>"""
import sys
from collections import Counter
import highspy, numpy as np
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1]); h.presolve()
lp = h.getPresolvedLp()
n, m = lp.num_col_, lp.num_row_
it = [int(t) for t in lp.integrality_]
print("列", n, "行", m, "種類", Counter(it))
A = lp.a_matrix_
st, idx, val = np.array(A.start_), np.array(A.index_), np.array(A.value_)
rl, ru, cl, cu, c = map(np.array, (lp.row_lower_, lp.row_upper_, lp.col_lower_, lp.col_upper_, lp.col_cost_))
rows = [[] for _ in range(m)]
for j in range(n):
    for k in range(st[j], st[j+1]):
        rows[idx[k]].append((j, val[k]))
imp = [j for j in range(n) if it[j] not in (0, 1)]
shape = Counter()
for j in imp:
    rs = [(idx[k], val[k]) for k in range(st[j], st[j+1])]
    shape[(len(rs), tuple(sorted("eq" if rl[i] == ru[i] else "in" for i, _ in rs)), c[j] != 0)] += 1
for k, v in shape.most_common(6):
    print("暗に整数の列の形 (非零, 行の種類, 費用あり)", k, v)
for j in imp[:3]:
    print("例 列", j, "境界", cl[j], cu[j], "費用", c[j])
    for k in range(st[j], st[j+1]):
        i = idx[k]
        print("   行", i, "係数", val[k], "範囲", rl[i], ru[i], "行の中身", [(q, b, it[q]) for q, b in rows[i]][:10])
