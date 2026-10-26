"""ENOMOTO の前処理後の問題で、暗に整数にならない連続列 (非零 2・不等式・費用なし) の例を見る"""
import sys
import highspy, numpy as np
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1])
lp = h.getLp(); n, m = lp.num_col_, lp.num_row_
A = lp.a_matrix_; st, idx, val = np.array(A.start_), np.array(A.index_), np.array(A.value_)
rl, ru, cl, cu, c = map(np.array, (lp.row_lower_, lp.row_upper_, lp.col_lower_, lp.col_upper_, lp.col_cost_))
it = [int(t) for t in lp.integrality_]
rows = [[] for _ in range(m)]
for j in range(n):
    for k in range(st[j], st[j+1]):
        rows[idx[k]].append((j, val[k]))
cnt = 0
for j in range(n):
    if it[j] or cl[j] == cu[j]:
        continue
    rs = [(idx[k], val[k]) for k in range(st[j], st[j+1])]
    if len(rs) == 2 and c[j] == 0 and all(rl[i] != ru[i] for i, _ in rs):
        cnt += 1
        if cnt <= 3:
            print("列", j, "境界", cl[j], cu[j])
            for i, a in rs:
                print("   行", i, "係数", a, "範囲", rl[i], ru[i], [(q, round(b, 6), it[q], round(c[q], 3)) for q, b in rows[i]][:8], "長さ", len(rows[i]))
