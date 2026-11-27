"""istanbul の前処理の差の試し (Python): 活動の範囲による境界の締め (FBBT) と、整数列の係数の締め (SCIP の consdataTightenCoefs の形) を
交互に回し、LP の値 (HiGHS、前処理なし) を測る。python fbbt_coef.py MPS [周の上限]"""
import sys, numpy as np, highspy
f = sys.argv[1]; R = int(sys.argv[2]) if len(sys.argv) > 2 else 50
h = highspy.Highs(); h.setOptionValue('output_flag', False); h.readModel(f)
lp = h.getLp(); n, m = lp.num_col_, lp.num_row_
isint = np.array([int(x) != 0 for x in lp.integrality_])
A = lp.a_matrix_; st = np.array(A.start_); idx = np.array(A.index_); val = np.array(A.value_, dtype=float)
rows = [[] for _ in range(m)]
for j in range(n):
    for k in range(st[j], st[j + 1]): rows[idx[k]].append([j, val[k]])
rl = np.array(lp.row_lower_, dtype=float); ru = np.array(lp.row_upper_, dtype=float)
lb = np.array(lp.col_lower_, dtype=float); ub = np.array(lp.col_upper_, dtype=float)
# 範囲の行 (両側) は 2 本の片側に分ける (係数の締めは片側ごと)
L = []  # (項, rhs)  の形 sum a x <= rhs
for i in range(m):
    if np.isfinite(ru[i]): L.append(([list(t) for t in rows[i]], ru[i]))
    if np.isfinite(rl[i]): L.append(([[j, -a] for j, a in rows[i]], -rl[i]))
def lp_val():
    hh = highspy.Highs(); hh.setOptionValue('output_flag', False); hh.setOptionValue('presolve', 'off')
    inf = highspy.kHighsInf
    hh.addVars(n, lb, ub)
    hh.changeColsCost(n, np.arange(n, dtype=np.int32), np.array(lp.col_cost_))
    for t, b in L:
        hh.addRow(-inf, b, len(t), np.array([j for j, _ in t], dtype=np.int32), np.array([a for _, a in t]))
    hh.run(); return hh.getInfo().objective_function_value
print('始め LP', round(lp_val(), 4))
EPS = 1e-9
for r in range(R):
    nb = nc = 0
    # FBBT
    for t, b in L:
        mn = [a * (lb[j] if a > 0 else ub[j]) for j, a in t]; tot = sum(mn)
        for (j, a), c in zip(t, mn):
            rest = tot - c; s = (b - rest) / a
            if a > 0 and s < ub[j] - 1e-7:
                s = np.floor(s + 1e-6) if isint[j] else s
                if s < ub[j] - 1e-7: ub[j] = max(s, lb[j]); nb += 1
            elif a < 0 and s > lb[j] + 1e-7:
                s = np.ceil(s - 1e-6) if isint[j] else s
                if s > lb[j] + 1e-7: lb[j] = min(s, ub[j]); nb += 1
    # 係数の締め (整数列だけ)
    for k, (t, b) in enumerate(L):
        mx = [a * (ub[j] if a > 0 else lb[j]) for j, a in t]; tot = sum(mx)
        if tot <= b + 1e-9: continue  # 冗長
        for q, ((j, a), c) in enumerate(zip(t, mx)):
            if not isint[j] or ub[j] - lb[j] < 0.5: continue
            maxR = tot - c
            if a > 0 and maxR < b - 1e-7:   # 下で冗長
                d = b - maxR; t[q][1] = a - d; b = maxR; L[k] = (t, b); nc += 1; break
            if a < 0 and maxR < b - a - 1e-7 and maxR > b:
                t[q][1] = -(maxR - b); nc += 1; break
    print(f'周 {r}: 境界 {nb}、係数 {nc}、LP {lp_val():.4f}', flush=True)
    if nb == 0 and nc == 0: break
