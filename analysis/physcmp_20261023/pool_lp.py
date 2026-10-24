"""HiGHS のカットプール (highs_trace.py の .pool*.json) を前処理後の問題 (MPS) に足して LP を解き、カットを形で分類し、
双対値が非零 (効いている) の本数を種類ごとに数える。python pool_lp.py <前処理後 mps> <pool json>"""
import json, sys
import numpy as np, highspy
from collections import Counter, defaultdict
pre, pj = sys.argv[1], sys.argv[2]
P = json.load(open(pj))
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(pre)
lp = h.getLp()
n = lp.num_col_
assert P["num_col"] == n, (P["num_col"], n)
isint = [lp.integrality_[j] != highspy.HighsVarType.kContinuous for j in range(n)] if len(lp.integrality_) else [False] * n
lo, up = lp.col_lower_, lp.col_upper_
binary = [isint[j] and lo[j] == 0 and up[j] == 1 for j in range(n)]
def kind(idx, val, lb, ub):
    k = len(idx)
    allbin = all(binary[j] for j in idx)
    allint = all(isint[j] for j in idx)
    unit = all(abs(abs(v) - 1) < 1e-9 for v in val)
    if allbin and unit and k == 2:
        return "2変数 ±1 (含意)"
    if allbin and unit:
        # 文字の和 <= 1 なら クリーク
        nneg = sum(1 for v in val if v < 0)
        if ub < 1e30 and abs(ub - (1 - nneg)) < 1e-9:
            return "クリーク (文字の和<=1)"
        return "0-1 ±1 (その他)"
    if allint:
        return "整数のみ (一般係数)"
    return "連続列を含む (MIR 型)"
h.setOptionValue("solve_relaxation", True); h.setOptionValue("presolve", "off")
h.run(); base = h.getInfo().objective_function_value
st, ix, vl = P["start"], P["index"], P["value"]
kinds = []
for c in range(len(st) - 1):
    idx = ix[st[c]:st[c + 1]]; val = vl[st[c]:st[c + 1]]
    kinds.append(kind(idx, val, P["lower"][c], P["upper"][c]))
    h.addRow(P["lower"][c], P["upper"][c], len(idx), np.array(idx, dtype=np.int32), np.array(val))
h.run()
info = h.getInfo(); sol = h.getSolution()
m0 = lp.num_row_
dual = sol.row_dual[m0:]
print(f"LP 前処理後のみ {base:.4f}  + プール {len(kinds)} 本 {info.objective_function_value:.4f}")
cnt = Counter(kinds); act = Counter(k for k, d in zip(kinds, dual) if abs(d) > 1e-7)
dsum = defaultdict(float)
for k, d, c in zip(kinds, dual, range(len(kinds))):
    rhs = P["upper"][c] if P["upper"][c] < 1e30 else P["lower"][c]
for k in cnt:
    print(f"  {k}: {cnt[k]} 本、双対非零 {act[k]}")
# 種類ごとに外したとき
for drop in cnt:
    g = highspy.Highs(); g.setOptionValue("output_flag", False); g.readModel(pre)
    for c in range(len(st) - 1):
        if kinds[c] == drop: continue
        idx = ix[st[c]:st[c + 1]]; val = vl[st[c]:st[c + 1]]
        g.addRow(P["lower"][c], P["upper"][c], len(idx), np.array(idx, dtype=np.int32), np.array(val))
    g.setOptionValue("solve_relaxation", True); g.setOptionValue("presolve", "off"); g.run()
    print(f"  {drop} を外す: {g.getInfo().objective_function_value:.4f}")
# 双対非零のカットを列名で表示
names = list(lp.col_names_) if len(lp.col_names_) else [f"c{j}" for j in range(n)]
x = sol.col_value
act_rows = []
for c in range(len(kinds)):
    if abs(dual[c]) > 1e-7:
        idx = ix[st[c]:st[c + 1]]; val = vl[st[c]:st[c + 1]]
        terms = " ".join(f"{v:+g}{names[j]}{'' if binary[j] else ('[I]' if isint[j] else '[C]')}" for j, v in zip(idx, val))
        print(f"  cut {c} [{kinds[c]}] dual {dual[c]:+.3f}: {P['lower'][c]:g} <= {terms} <= {P['upper'][c]:g}")
        act_rows.append(c)
json.dump({"active": act_rows, "names": [[names[j] for j in ix[st[c]:st[c+1]]] for c in act_rows]}, open(pj + ".active.json", "w"))
