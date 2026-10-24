"""ENOMOTO のカット入りの LP に、HiGHS のプールのカットを種類ごとに足した LP の値。
python inject_cls.py <cutlp mps> <pool json>  (種類: pool_lp2 と同じ。「破るものだけ」「双対非零のうち破るものだけ」も)"""
import json, sys
import numpy as np, highspy
from collections import defaultdict
mps, pj = sys.argv[1], sys.argv[2]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(mps); lp = h.getLp()
n = lp.num_col_; lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
integ = list(lp.integrality_); isint = [integ[j] != highspy.HighsVarType.kContinuous for j in range(n)] if integ else [True] * n
h.run(); base = h.getInfo().objective_function_value; x = np.array(h.getSolution().col_value)
binary = [isint[j] and lo[j] == 0 and up[j] == 1 for j in range(n)]
def kind(idx, val, ub):
    k = len(idx); allbin = all(binary[j] for j in idx); unit = all(abs(abs(v) - 1) < 1e-9 for v in val)
    if allbin and unit and k == 2: return "2変数"
    if allbin and unit:
        nneg = sum(1 for v in val if v < 0)
        return "クリーク" if ub < 1e30 and abs(ub - (1 - nneg)) < 1e-9 else "0-1±1他"
    return "一般係数"
P = json.load(open(pj)); S, I, V = P["start"], P["index"], P["value"]
act = set(json.load(open(pj + ".active.json"))["active"])
groups = defaultdict(list)
for c in range(len(S) - 1):
    idx = I[S[c]:S[c + 1]]; val = V[S[c]:S[c + 1]]
    a = float(np.dot(x[idx], val)); v = max(P["lower"][c] - a, a - P["upper"][c])
    k = kind(idx, val, P["upper"][c])
    groups[k].append(c); groups["全部"].append(c)
    if v > 1e-6:
        groups[k + "・破る"].append(c); groups["全部・破る"].append(c)
        if c in act: groups[k + "・破る・HiGHS双対非零"].append(c); groups["全部・破る・HiGHS双対非零"].append(c)
print(f"{mps}: LP {base:.4f}")
for k in sorted(groups):
    g = highspy.Highs(); g.setOptionValue("output_flag", False); g.readModel(mps)
    for c in groups[k]:
        idx = I[S[c]:S[c + 1]]; val = V[S[c]:S[c + 1]]
        g.addRow(P["lower"][c], P["upper"][c], len(idx), np.array(idx, dtype=np.int32), np.array(val))
    g.run(); print(f"  + {k} ({len(groups[k])} 本): {g.getInfo().objective_function_value:.4f}", flush=True)
