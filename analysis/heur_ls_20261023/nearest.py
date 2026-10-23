"""点に最も近い実行可能解 (変える列の数 = 0-1 列の値の違い + 一般整数列の |差| の和、の最小) を HiGHS で求める。
python -I nearest.py <dump> <点> <秒> [出力]
変わった列を、違反行との関係 (違反行の列か、そこから何段か) で分けて出す。"""
import sys, numpy as np, highspy, scipy.sparse as sp, collections
d, ptf, T = sys.argv[1], sys.argv[2], float(sys.argv[3])
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f"{d}/prob.mps"); lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
A = sp.csc_matrix((lp.a_matrix_.value_, lp.a_matrix_.index_, lp.a_matrix_.start_), shape=(m, n)); R = A.tocsr()
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_); rlo, rup = np.array(lp.row_lower_), np.array(lp.row_upper_)
integ = np.array([int(t) == 1 for t in lp.integrality_])
x = np.array([float(l) for l in open(ptf)]); act = R @ x
# 目的を「点からの距離」に置き換える
cost = np.zeros(n)
binc = integ & (lo == 0) & (up == 1)
cost[binc & (x > 0.5)] = -1.0
cost[binc & (x < 0.5)] = 1.0
h.changeColsCost(n, np.arange(n, dtype=np.int32), cost)
others = [j for j in range(n) if lo[j] < up[j] and not binc[j]]
# 一般整数・連続列: x_j - p_j + q_j = x0_j、p,q >= 0、費用 1 (連続列は 1e-3)
for j in others:
    w = 1.0 if integ[j] else 1e-3
    k = h.getNumCol()
    h.addVars(2, np.array([0.0, 0.0]), np.array([highspy.kHighsInf, highspy.kHighsInf]))
    h.changeColsCost(2, np.array([k, k + 1], dtype=np.int32), np.array([w, w]))
    h.addRow(x[j], x[j], 3, np.array([j, k, k + 1], dtype=np.int32), np.array([1.0, -1.0, 1.0]))
h.setOptionValue("time_limit", T); h.setOptionValue("threads", 4); h.setOptionValue("output_flag", False)
h.run()
st = h.modelStatusToString(h.getModelStatus())
y = np.array(h.getSolution().col_value[:n])
info = h.getInfo()
print(f"最も近い実行可能解: {st}、距離 {info.objective_function_value + (x[binc] > 0.5).sum():.4g} (下界 {info.mip_dual_bound + (x[binc] > 0.5).sum():.4g})")
if st not in ("Optimal", "Time limit reached") or not np.isfinite(info.objective_function_value):
    sys.exit()
ch = np.where(np.abs(y - x) > 1e-6)[0]
print(f"変わった列 {len(ch)} (0-1 {binc[ch].sum()}、一般整数 {(integ & ~binc)[ch].sum()}、連続 {(~integ)[ch].sum()})")
tol = 1e-6
V = [i for i in range(m) if act[i] > rup[i] + tol or act[i] < rlo[i] - tol]
# 違反行からの段 (行-列の二部グラフでの距離)
dist = {}
frontier_rows = set(V); seen_rows = set(V); lvl = 0
while frontier_rows and lvl < 6:
    cols = set()
    for i in frontier_rows:
        for j in R.indices[R.indptr[i]:R.indptr[i+1]]:
            if j not in dist: dist[j] = lvl; cols.add(j)
    nr = set()
    for j in cols:
        for i in A.indices[A.indptr[j]:A.indptr[j+1]]:
            if i not in seen_rows: seen_rows.add(i); nr.add(i)
    frontier_rows = nr; lvl += 1
print("変わった列の、違反行からの段:", dict(collections.Counter(dist.get(j, 99) for j in ch)))
print("目的値 (元の費用):", float(np.array(lp.col_cost_) @ y))
if len(sys.argv) > 4:
    open(sys.argv[4], "w").write("".join(f"{v:.17e}\n" for v in y))
for j in ch[:40]:
    print(f"  c{j} {'bin' if binc[j] else ('int' if integ[j] else 'cont')} {x[j]:g} -> {y[j]:g} 段 {dist.get(j, 99)}")
