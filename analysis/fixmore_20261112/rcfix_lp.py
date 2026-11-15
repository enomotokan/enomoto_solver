"""LP 緩和 (カットなし、元の問題) の被約費用で、打ち切り値 U ごとに固定できる整数列を数える。
使い方: python rcfix_lp.py MPS U [U ...]   (単体法の双対と、内点法 (クロスオーバーなし) の双対の両方)"""
import sys, numpy as np, highspy
mps = sys.argv[1]; Us = [float(u) for u in sys.argv[2:]]
for solver in ("simplex", "ipm"):
    h = highspy.Highs(); h.setOptionValue("output_flag", False)
    h.readModel(mps); lp = h.getLp()
    n = lp.num_col_
    integ = np.array([int(x) for x in lp.integrality_]) == 1
    h.changeColsIntegrality(n, np.arange(n, dtype=np.int32), np.zeros(n, dtype=np.uint8))
    h.setOptionValue("solver", solver)
    if solver == "ipm": h.setOptionValue("run_crossover", "off")
    h.run(); z = h.getInfo().objective_function_value
    sol = h.getSolution(); d = np.array(sol.col_dual); x = np.array(sol.col_value)
    lo = np.array(lp.col_lower_); up = np.array(lp.col_upper_); w = up - lo
    free = integ & (w > 0.5)
    for U in Us:
        g = U - z
        fix = free & (np.abs(d) * w > g + 1e-9)
        print(f"{solver} z={z:.4f} U={U} gap={g:.3f} free_int={free.sum()} rcfix={fix.sum()} ({100*fix.sum()/max(1,free.sum()):.1f}%)")
