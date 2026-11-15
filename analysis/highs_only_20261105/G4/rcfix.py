"""satellites2-60-fs: 元の問題の LP 緩和の被約費用で、打ち切り値 (暫定解 - 1) から固定できる列の数を数え、
固定を行の伝播 (活動の上下限) で広げた数を出す (G4 の調べ)。使い方: python -I rcfix.py MPS 暫定解 [暫定解 ...]"""
import sys, numpy as np, highspy
mps = sys.argv[1]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(mps)
lp = h.getLp(); n, m = lp.num_col_, lp.num_row_
integ = np.array([int(t) for t in lp.integrality_]) if len(lp.integrality_) else np.zeros(n, int)
h.changeColsIntegrality(n, np.arange(n, dtype=np.int32), np.zeros(n, dtype=np.uint8))
h.run(); sol = h.getSolution(); z = h.getInfo().objective_function_value
rc = np.array(sol.col_dual); x = np.array(sol.col_value)
lo = np.array(lp.col_lower_); up = np.array(lp.col_upper_)
st = np.array(lp.a_matrix_.start_); ix = np.array(lp.a_matrix_.index_); va = np.array(lp.a_matrix_.value_)
CR = ix; CC = np.repeat(np.arange(n), np.diff(st)); CV = va
rl = np.array(lp.row_lower_); ru = np.array(lp.row_upper_)
print(f"LP {z:.4f}, cols {n} (int {int((integ>0).sum())}), rows {m}")
def propagate(lo, up, rounds=50):
    lo, up = lo.copy(), up.copy()
    for _ in range(rounds):
        ch = 0
        BIG = 1e30
        l_ = np.where(np.isfinite(lo), lo, -BIG); u_ = np.where(np.isfinite(up), up, BIG)
        minact = np.zeros(m); maxact = np.zeros(m)
        np.add.at(minact, CR, np.where(CV > 0, CV * l_[CC], CV * u_[CC]))
        np.add.at(maxact, CR, np.where(CV > 0, CV * u_[CC], CV * l_[CC]))
        for r, j, a in zip(CR.tolist(), CC.tolist(), CV.tolist()):
            if lo[j] == up[j]:
                continue
            # 行 r の上限から x_j の上限・下限
            if ru[r] < 1e20 and minact[r] > -1e20:
                rest = minact[r] - (a * lo[j] if a > 0 else a * up[j])
                b = (ru[r] - rest) / a
                if a > 0 and b < up[j] - 1e-9:
                    nb = np.floor(b + 1e-9) if integ[j] else b
                    if nb < up[j]: up[j] = max(nb, lo[j]); ch += 1
                elif a < 0 and b > lo[j] + 1e-9:
                    nb = np.ceil(b - 1e-9) if integ[j] else b
                    if nb > lo[j]: lo[j] = min(nb, up[j]); ch += 1
            if rl[r] > -1e20 and maxact[r] < 1e20:
                rest = maxact[r] - (a * up[j] if a > 0 else a * lo[j])
                b = (rl[r] - rest) / a
                if a > 0 and b > lo[j] + 1e-9:
                    nb = np.ceil(b - 1e-9) if integ[j] else b
                    if nb > lo[j]: lo[j] = min(nb, up[j]); ch += 1
                elif a < 0 and b < up[j] - 1e-9:
                    nb = np.floor(b + 1e-9) if integ[j] else b
                    if nb < up[j]: up[j] = max(nb, lo[j]); ch += 1
        if ch == 0:
            break
    return lo, up
fx0 = ((lo == up) & (integ > 0)).sum()
lo1, up1 = propagate(lo, up)
base = ((lo1 == up1) & (integ > 0)).sum()
print("fixed int at start", fx0, "after propagation", base)
for inc in map(float, sys.argv[2:]):
    gap = (inc - 1) - z
    l2, u2 = lo1.copy(), up1.copy()
    k = 0
    for j in range(n):
        if integ[j] and l2[j] < u2[j]:
            if rc[j] > gap and x[j] <= l2[j] + 1e-9: u2[j] = l2[j]; k += 1
            elif rc[j] < -gap and x[j] >= u2[j] - 1e-9: l2[j] = u2[j]; k += 1
    l3, u3 = propagate(l2, u2)
    print(f"incumbent {inc}: cutoff gap {gap:.3f}, rc-fixed {k}, fixed int after propagation {((l3 == u3) & (integ > 0)).sum()} of {(integ>0).sum()}")
