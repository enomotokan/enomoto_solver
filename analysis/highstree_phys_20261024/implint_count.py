"""ENOMOTO の前処理後の問題 (MPS) で、ENOMOTO の implied_integers と同じ規則で暗に整数になる連続列を数える。
python -I implint_count.py <mps>"""
import sys
import highspy
import numpy as np
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1])
lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
A = lp.a_matrix_
start, idx, val = np.array(A.start_), np.array(A.index_), np.array(A.value_)
cl, cu, rl, ru, c = map(np.array, (lp.col_lower_, lp.col_upper_, lp.row_lower_, lp.row_upper_, lp.col_cost_))
isint = np.array([int(t) != 0 for t in lp.integrality_]) if len(lp.integrality_) else np.zeros(n, bool)
cols = [list(zip(idx[start[j]:start[j+1]], val[start[j]:start[j+1]])) for j in range(n)]
rows = [[] for _ in range(m)]
for j in range(n):
    for i, a in cols[j]:
        rows[i].append((j, a))
integral = lambda v: abs(v - round(v)) <= 1e-9 * max(abs(v), 1.0)
fixed = cl == cu
mark = isint.copy()
cont = [j for j in range(n) if not isint[j] and not fixed[j]]
print("連続列 (固定でない)", len(cont))
while True:
    ch = False
    for j in cont:
        if mark[j]:
            continue
        ok = any(rl[i] == ru[i] and integral(rl[i] / a) and all(k == j or (mark[k] and integral(b / a)) for k, b in rows[i]) for i, a in cols[j])
        if not ok and len(cols[j]) == 1 and c[j] != 0:
            i, a = cols[j][0]
            side = rl[i] if (c[j] > 0) == (a > 0) else ru[i]
            ok = all((not np.isfinite(v)) or integral(v) for v in (cl[j], cu[j])) and ((not np.isfinite(side)) or integral(side / a)) and all(k == j or (mark[k] and integral(b / a)) for k, b in rows[i])
        if ok:
            mark[j] = True; ch = True
    if not ch:
        break
print("暗に整数と判定", sum(1 for j in cont if mark[j]))
# 残った連続列の形
from collections import Counter
ctr = Counter()
for j in cont:
    if mark[j]:
        continue
    ctr[(len(cols[j]), c[j] != 0, tuple(sorted(set("eq" if rl[i] == ru[i] else "in" for i, _ in cols[j]))))] += 1
for k, v in ctr.most_common(8):
    print("残り: 非零", k[0], "費用あり" if k[1] else "費用なし", k[2], v)
j0 = next(j for j in cont if not mark[j])
print("例 列", j0, "境界", cl[j0], cu[j0], "費用", c[j0], [(i, a, rl[i], ru[i], len(rows[i])) for i, a in cols[j0]][:4])
i0 = cols[j0][0][0]
print("その行", [(k, b, bool(isint[k]), bool(mark[k])) for k, b in rows[i0]][:12])
# HiGHS の双対側の判定 (HPresolve::isImpliedInteger): 列が出る全部の行で、他の列が (暗に) 整数、b/a と有限の左辺・右辺 / a が整数、
# 列の境界が整数か無限なら暗に整数 (費用・行の数は問わない)。primal の等式の判定と合わせて繰り返す
mark2 = isint.copy()
while True:
    ch = False
    for j in cont:
        if mark2[j]:
            continue
        ok = any(rl[i] == ru[i] and integral(rl[i] / a) and all(k == j or (mark2[k] and integral(b / a)) for k, b in rows[i]) for i, a in cols[j])
        if not ok and all(rl[i] != ru[i] for i, _ in cols[j]):
            ok = all((not np.isfinite(v)) or integral(v) for v in (cl[j], cu[j])) and all(
                ((not np.isfinite(rl[i])) or integral(rl[i] / a)) and ((not np.isfinite(ru[i])) or integral(ru[i] / a)) and all(k == j or (mark2[k] and integral(b / a)) for k, b in rows[i])
                for i, a in cols[j])
        if ok:
            mark2[j] = True; ch = True
    if not ch:
        break
print("HiGHS の規則で暗に整数", sum(1 for j in cont if mark2[j]))
