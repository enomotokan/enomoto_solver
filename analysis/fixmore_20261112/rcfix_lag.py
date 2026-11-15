"""LP 緩和 (カットなし、元の問題) の双対 y からラグランジュ下界 L(y) と被約費用 c - A^T y を作り、打ち切り値 U ごとに
被約費用で固定できる整数列を数える (近似の双対でも L(y) が正しい下界なので固定は正しい)。
行の双対の符号が行の有限な側と合わなければ 0 にする (射影)。
使い方: python rcfix_lag.py MPS U [U ...]"""
import sys, numpy as np, highspy, scipy.sparse as sp
mps = sys.argv[1]; Us = [float(u) for u in sys.argv[2:]]
for solver in ("simplex", "ipm"):
    h = highspy.Highs(); h.setOptionValue("output_flag", False)
    h.readModel(mps); lp = h.getLp(); n = lp.num_col_; m = lp.num_row_
    integ = np.array([int(x) for x in lp.integrality_]) == 1
    h.changeColsIntegrality(n, np.arange(n, dtype=np.int32), np.zeros(n, dtype=np.uint8))
    h.setOptionValue("solver", solver)
    if solver == "ipm": h.setOptionValue("run_crossover", "off")
    h.run(); z = h.getInfo().objective_function_value
    sol = h.getSolution(); y = np.array(sol.row_dual)
    a = lp.a_matrix_
    A = sp.csc_matrix((np.array(a.value_), np.array(a.index_), np.array(a.start_)), shape=(m, n))
    c = np.array(lp.col_cost_); lo = np.array(lp.col_lower_); up = np.array(lp.col_upper_)
    rl = np.array(lp.row_lower_); ru = np.array(lp.row_upper_)
    y = np.where((y > 0) & ~np.isfinite(rl), 0.0, y); y = np.where((y < 0) & ~np.isfinite(ru), 0.0, y)
    d = c - A.T @ y
    rowpart = np.where(y > 0, y * np.where(np.isfinite(rl), rl, 0), y * np.where(np.isfinite(ru), ru, 0)).sum()
    colb = np.where(d > 0, lo, up)
    bad = ~np.isfinite(colb) & (d != 0)
    colpart = np.where(d != 0, d * np.where(np.isfinite(colb), colb, 0), 0).sum()
    L = rowpart + colpart if not bad.any() else -np.inf
    w = up - lo; free = integ & (w > 0.5)
    for U in Us:
        g = U - L
        fix = free & np.isfinite(w) & (np.abs(d) * w > g + 1e-6)
        print(f"{solver} z={z:.6f} L={L:.6f} badcols={bad.sum()} U={U} gap={g:.3f} free_int={free.sum()} rcfix={fix.sum()} ({100*fix.sum()/max(1,free.sum()):.1f}%)", flush=True)
