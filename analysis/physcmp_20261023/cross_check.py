"""ENOMOTO の前処理後の問題 (dump、列 C<j> = 元の列 j) の LP 解 (+ HiGHS のカット) を HiGHS の前処理後の問題の列 (名前 c<j+1>) に写し、
HiGHS の前処理後の行・HiGHS のカットのどれを破るかを数える。python cross_check.py <HiGHS 前処理後 mps> <pool json> <ENOMOTO dump mps>"""
import json, sys
import numpy as np, highspy
from collections import Counter
hp, pj, ep = sys.argv[1:4]
P = json.load(open(pj))
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(hp)
hl = h.getLp(); hn = list(hl.col_names_); hm = hl.num_row_
e = highspy.Highs(); e.setOptionValue("output_flag", False); e.readModel(ep)
el = e.getLp(); en = {nm: j for j, nm in enumerate(el.col_names_)}
emap = [en["C%d" % (int(nm[1:]) - 1)] for nm in hn]
e.setOptionValue("solve_relaxation", True); e.setOptionValue("presolve", "off"); e.run()
x = np.array(e.getSolution().col_value)
xh = x[emap]
print("ENOMOTO LP", e.getInfo().objective_function_value, "HiGHS objective of projection", float(np.dot(hl.col_cost_, xh) + hl.offset_))
# HiGHS 前処理後の列の境界
lo, up = np.array(hl.col_lower_), np.array(hl.col_upper_)
print("col bound viol", int(((xh < lo - 1e-6) | (xh > up + 1e-6)).sum()))
# 行
A = h.getLp().a_matrix_
st, ix, vl = np.array(A.start_), np.array(A.index_), np.array(A.value_)
act = np.zeros(hm)
for j in range(hl.num_col_):
    for p in range(st[j], st[j + 1]):
        act[ix[p]] += vl[p] * xh[j]
rl, ru = np.array(hl.row_lower_), np.array(hl.row_upper_)
viol = np.maximum(rl - act, act - ru)
bad = np.where(viol > 1e-6)[0]
print("HiGHS presolved rows violated:", len(bad), "of", hm, " max viol", viol.max())
# 破った行の形
rows = [[] for _ in range(hm)]
for j in range(hl.num_col_):
    for p in range(st[j], st[j + 1]):
        rows[ix[p]].append((j, vl[p]))
isint = [hl.integrality_[j] != highspy.HighsVarType.kContinuous for j in range(hl.num_col_)]
kinds = Counter()
for i in bad[:2000]:
    r = rows[i]; k = len(r)
    kinds[(k if k < 6 else "6+", all(isint[j] for j, _ in r), "eq" if rl[i] == ru[i] else "ineq")] += 1
print(kinds.most_common(12))
for i in bad[:12]:
    r = rows[i]
    print(f"  {rl[i]:g} <= " + " ".join(f"{v:+g}{hn[j]}({xh[j]:.2f})" for j, v in r[:12]) + f" <= {ru[i]:g}  act {act[i]:.3f}")
