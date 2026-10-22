# 行の長さ・等式の 2 項の係数比・列シングルトンを集計する
import sys, highspy, numpy as np, collections
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1]); lp = h.getLp()
m, n = lp.num_row_, lp.num_col_
st = np.array(lp.a_matrix_.start_); idx = np.array(lp.a_matrix_.index_); val = np.array(lp.a_matrix_.value_)
rows = [[] for _ in range(m)]
for j in range(n):
    for k in range(st[j], st[j+1]): rows[idx[k]].append((j, val[k]))
rl, ru = np.array(lp.row_lower_), np.array(lp.row_upper_)
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
eq = rl == ru
print("row len (eq)", collections.Counter(len(rows[i]) for i in range(m) if eq[i]).most_common(10))
print("row len (ineq)", collections.Counter(len(rows[i]) for i in range(m) if not eq[i]).most_common(10))
d = collections.Counter()
for i in range(m):
    if eq[i] and len(rows[i]) == 2:
        (a, x), (b, y) = rows[i]
        d[(x, y, rl[i], (lo[a], up[a]), (lo[b], up[b]))] += 1
print("eq doubletons", d.most_common(10))
collen = np.diff(st)
cs = collections.Counter()
for j in range(n):
    if collen[j] == 1:
        i = idx[st[j]]; cs[(eq[i], len(rows[i]), lp.col_cost_[j] != 0, (lo[j], up[j]))] += 1
print("col singletons (eq?, rowlen, cost!=0, bounds)", cs.most_common(10))
print("cost nonzero", (np.array(lp.col_cost_) != 0).sum())
