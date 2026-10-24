"""pool_lp.py と同じ分類を、固定列の多い ENOMOTO の dump でも速く行う (固定列を右辺に移して除いた LP を作る)。
python pool_lp2.py <mps> <pool json>  (プールの列番号は <mps> の列番号)"""
import json, sys
import numpy as np, highspy
from collections import Counter
mps, pj = sys.argv[1], sys.argv[2]
P = json.load(open(pj))
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(mps); lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
free = np.where(lo < up)[0]; pos = -np.ones(n, dtype=np.int64); pos[free] = np.arange(len(free))
fixv = np.where(lo == up, lo, 0.0)
A = lp.a_matrix_; st, ix, vl = np.array(A.start_), np.array(A.index_), np.array(A.value_)
rows = [[] for _ in range(m)]; shift = np.zeros(m)
for j in range(n):
    for p in range(st[j], st[j + 1]):
        if pos[j] >= 0: rows[ix[p]].append((int(pos[j]), vl[p]))
        else: shift[ix[p]] += vl[p] * fixv[j]
integ = list(lp.integrality_)
isint = [integ[j] != highspy.HighsVarType.kContinuous for j in range(n)]
names = list(lp.col_names_)
cost = np.array(lp.col_cost_); off = lp.offset_ + float(np.dot(cost, fixv))
rl = np.array(lp.row_lower_) - shift; ru = np.array(lp.row_upper_) - shift


def build(extra):
    g = highspy.Highs(); g.setOptionValue("output_flag", False)
    g.addVars(len(free), lo[free], up[free])
    g.changeColsCost(len(free), np.arange(len(free), dtype=np.int32), cost[free])
    for i in range(m):
        r = rows[i]
        g.addRow(rl[i], ru[i], len(r), np.array([a for a, _ in r], dtype=np.int32), np.array([b for _, b in r]))
    for (lb, ub, idx, val) in extra:
        g.addRow(lb, ub, len(idx), np.array(idx, dtype=np.int32), np.array(val))
    g.run(); return g


binary = [isint[j] and lo[j] == 0 and up[j] == 1 for j in range(n)]


def kind(idx, val, ub):
    k = len(idx); allbin = all(binary[j] for j in idx); unit = all(abs(abs(v) - 1) < 1e-9 for v in val)
    if allbin and unit and k == 2: return "2変数 ±1 (含意)"
    if allbin and unit:
        nneg = sum(1 for v in val if v < 0)
        if ub < 1e30 and abs(ub - (1 - nneg)) < 1e-9: return "クリーク (文字の和<=1)"
        return "0-1 ±1 (その他)"
    if all(isint[j] for j in idx): return "整数のみ (一般係数)"
    return "連続列を含む (MIR 型)"


cuts, kinds = [], []
S, I, V = P["start"], P["index"], P["value"]
for c in range(len(S) - 1):
    idx = I[S[c]:S[c + 1]]; val = V[S[c]:S[c + 1]]
    sh = sum(v * fixv[j] for j, v in zip(idx, val) if pos[j] < 0)
    fi = [(int(pos[j]), v) for j, v in zip(idx, val) if pos[j] >= 0]
    lb = P["lower"][c] - sh if P["lower"][c] > -1e30 else P["lower"][c]
    ub = P["upper"][c] - sh if P["upper"][c] < 1e30 else P["upper"][c]
    cuts.append((lb, ub, [a for a, _ in fi], [b for _, b in fi])); kinds.append(kind(idx, val, P["upper"][c]))
g0 = build([]); base = g0.getInfo().objective_function_value + off
g = build(cuts); val = g.getInfo().objective_function_value + off
dual = np.array(g.getSolution().row_dual)[m:]
print(f"LP 問題のみ {base:.4f}  + プール {len(cuts)} 本 {val:.4f}", flush=True)
cnt = Counter(kinds); act = Counter(k for k, d in zip(kinds, dual) if abs(d) > 1e-7)
for k in cnt: print(f"  {k}: {cnt[k]} 本、双対非零 {act[k]}", flush=True)
fn = [names[j] for j in free]
act_rows = [c for c in range(len(cuts)) if abs(dual[c]) > 1e-7]
json.dump({"active": act_rows}, open(pj + ".active.json", "w"))
# 双対非零のカットだけを足した LP
ga = build([cuts[c] for c in act_rows])
print(f"  双対非零の {len(act_rows)} 本だけ: {ga.getInfo().objective_function_value + off:.4f}", flush=True)
for drop in cnt:
    gg = build([c for c, k in zip(cuts, kinds) if k != drop])
    print(f"  {drop} を外す: {gg.getInfo().objective_function_value + off:.4f}", flush=True)
for c in act_rows[:80]:
    lb, ub, idx, vv = cuts[c]
    print(f"  cut {c} [{kinds[c]}] dual {dual[c]:+.3f}: {lb:g} <= " + " ".join(f"{v:+g}{fn[j]}" for j, v in zip(idx, vv)) + f" <= {ub:g}")
