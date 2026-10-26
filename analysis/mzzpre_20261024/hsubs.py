"""HiGHS の前処理で消えた列のうち、残った 1 列と同値 (x_j = x_k / x_j = 1 - x_k) な組を、前処理後の LP 緩和の解
(乱数の目的 6 個) を後処理で戻して探す: python hsubs.py <mps> <固定の tsv> [キー=値 ...]
組の等式と固定を元の問題に足した LP 緩和の値を出す。組を logs/hsubs_pairs.tsv に書く。"""
import sys
import numpy as np
import highspy
mps, fixtsv = sys.argv[1], sys.argv[2]
h = highspy.Highs(); h.setOptionValue("output_flag", False)
for kv in sys.argv[3:]:
    k, v = kv.split("="); h.setOptionValue(k, int(v))
h.readModel(mps)
l0 = h.getLp(); names = list(l0.col_names_); n0 = {n: j for j, n in enumerate(names)}; N = l0.num_col_
h.presolve(); p = h.getPresolvedLp(); kept = [n0[n] for n in p.col_names_]; keptset = set(kept)
rng = np.random.default_rng(1)
X = []
for t in range(6):
    lp = h.getPresolvedLp(); lp.integrality_ = []
    lp.col_cost_ = np.array(lp.col_cost_) if t == 0 else rng.normal(size=lp.num_col_)
    q = highspy.Highs(); q.setOptionValue("output_flag", False); q.passModel(lp); q.run()
    h.postsolve(q.getSolution()); X.append(np.array(h.getSolution().col_value))
X = np.array(X)  # 6 x N
removed = [j for j in range(N) if j not in keptset]
var = [j for j in removed if np.ptp(X[:, j]) > 1e-9]
pairs = []
Kx = X[:, kept]
for j in var:
    d1 = np.abs(Kx - X[:, [j]]).max(axis=0); d2 = np.abs(Kx - (1 - X[:, [j]])).max(axis=0)
    a, b = d1.argmin(), d2.argmin()
    if d1[a] < 1e-7: pairs.append((j, kept[a], False))
    elif d2[b] < 1e-7: pairs.append((j, kept[b], True))
print("removed", len(removed), "varying", len(var), "pairs", len(pairs))
with open("logs/hsubs_pairs.tsv", "w") as f:
    for j, k, neg in pairs: f.write(f"{names[j]}\t{names[k]}\t{int(neg)}\n")
lo = np.array(l0.col_lower_); up = np.array(l0.col_upper_)
for line in open(fixtsv):
    nm, kind, v, _ = line.split("\t")
    if kind == "fix": j = n0[nm]; lo[j] = up[j] = round(float(v))
def lpval(lo, up, pairs):
    q = highspy.Highs(); q.setOptionValue("output_flag", False); q.readModel(mps)
    lp = q.getLp(); lp.integrality_ = []; lp.col_lower_ = lo; lp.col_upper_ = up; q.passModel(lp)
    for j, k, neg in pairs:
        q.addRow(1.0 if neg else 0.0, 1.0 if neg else 0.0, 2, np.array([j, k], dtype=np.int32), np.array([1.0, 1.0 if neg else -1.0]))
    q.run(); return q.getInfo().objective_function_value
print("pairs only", lpval(np.array(l0.col_lower_), np.array(l0.col_upper_), pairs))
print("fixes + pairs", lpval(lo, up, pairs))
