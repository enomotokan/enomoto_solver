"""istanbul の前処理の差の試し (Python): 0-1 列ごとに 0・1 に固定して FBBT を回し、両側の境界の和 (hull) で連続列の大域の境界を締める (プロービング)。
周を重ね、その後に整数列の係数の締めをする。LP の値は HiGHS (前処理なし)。python probe_hull.py MPS [周の上限]"""
import sys, numpy as np, highspy
f = sys.argv[1]; R = int(sys.argv[2]) if len(sys.argv) > 2 else 30
h = highspy.Highs(); h.setOptionValue('output_flag', False); h.readModel(f)
lp = h.getLp(); n, m = lp.num_col_, lp.num_row_
isint = np.array([int(x) != 0 for x in lp.integrality_])
A = lp.a_matrix_; st = np.array(A.start_); idx = np.array(A.index_); val = np.array(A.value_, dtype=float)
rows = [[] for _ in range(m)]
for j in range(n):
    for k in range(st[j], st[j + 1]): rows[idx[k]].append((j, val[k]))
rl = np.array(lp.row_lower_, dtype=float); ru = np.array(lp.row_upper_, dtype=float)
LB = np.array(lp.col_lower_, dtype=float); UB = np.array(lp.col_upper_, dtype=float)
L = []
for i in range(m):
    if np.isfinite(ru[i]): L.append(([list(t) for t in rows[i]], ru[i]))
    if np.isfinite(rl[i]): L.append(([[j, -a] for j, a in rows[i]], -rl[i]))
colrows = [[] for _ in range(n)]
for k, (t, b) in enumerate(L):
    for j, _ in t: colrows[j].append(k)
def fbbt(lb, ub, start=None, maxit=10**7):
    """行のキューで境界を伝える。矛盾なら False"""
    q = list(range(len(L))) if start is None else list({k for j in start for k in colrows[j]})
    inq = set(q); it = 0
    while q and it < maxit:
        k = q.pop(); inq.discard(k); it += 1; t, b = L[k]
        mn = [a * (lb[j] if a > 0 else ub[j]) for j, a in t]; tot = sum(mn)
        if tot > b + 1e-6: return False
        for (j, a), c in zip(t, mn):
            s = (b - (tot - c)) / a; ch = False
            if a > 0 and s < ub[j] - 1e-7:
                s = np.floor(s + 1e-6) if isint[j] else s
                if s < ub[j] - 1e-7:
                    if s < lb[j] - 1e-6: return False
                    ub[j] = max(s, lb[j]); ch = True
            elif a < 0 and s > lb[j] + 1e-7:
                s = np.ceil(s - 1e-6) if isint[j] else s
                if s > lb[j] + 1e-7:
                    if s > ub[j] + 1e-6: return False
                    lb[j] = min(s, ub[j]); ch = True
            if ch:
                for kk in colrows[j]:
                    if kk not in inq: q.append(kk); inq.add(kk)
    return True
def lp_val(lb, ub, LL):
    hh = highspy.Highs(); hh.setOptionValue('output_flag', False); hh.setOptionValue('presolve', 'off')
    inf = highspy.kHighsInf; hh.addVars(n, lb, ub)
    hh.changeColsCost(n, np.arange(n, dtype=np.int32), np.array(lp.col_cost_))
    for t, b in LL: hh.addRow(-inf, b, len(t), np.array([j for j, _ in t], dtype=np.int32), np.array([a for _, a in t]))
    hh.run(); return hh.getInfo().objective_function_value
def coef(lb, ub):
    LL = [([list(x) for x in t], b) for t, b in L]; nc = 0
    for k, (t, b) in enumerate(LL):
        for q in range(len(t)):
            j, a = t[q]
            if not isint[j] or ub[j] - lb[j] < 0.5: continue
            tot = sum(aa * (ub[jj] if aa > 0 else lb[jj]) for jj, aa in t)
            if tot <= b + 1e-9: break
            maxR = tot - a * (ub[j] if a > 0 else lb[j])
            if a > 0 and maxR < b - 1e-7: t[q][1] = a - (b - maxR); b = maxR; nc += 1
            elif a < 0 and maxR > b and maxR < b - a - 1e-7: t[q][1] = -(maxR - b); nc += 1
        LL[k] = (t, b)
    return LL, nc
lb, ub = LB.copy(), UB.copy(); fbbt(lb, ub)
print('FBBT の後 LP', round(lp_val(lb, ub, L), 4), flush=True)
bins = [j for j in range(n) if isint[j] and lb[j] == 0 and ub[j] == 1]
for r in range(R):
    nl = lb.copy(); nu = ub.copy(); fixed = 0
    for j in bins:
        if ub[j] - lb[j] < 0.5: continue
        res = []
        for v in (0.0, 1.0):
            l2, u2 = lb.copy(), ub.copy(); l2[j] = u2[j] = v
            res.append((l2, u2) if fbbt(l2, u2, start=[j]) else None)
        if res[0] is None and res[1] is None: print('矛盾'); sys.exit()
        if res[0] is None or res[1] is None:
            l2, u2 = res[1] if res[0] is None else res[0]; lb[:] = l2; ub[:] = u2; fixed += 1; continue
        hl = np.minimum(res[0][0], res[1][0]); hu = np.maximum(res[0][1], res[1][1])
        lb[:] = np.maximum(lb, hl); ub[:] = np.minimum(ub, hu)
    fbbt(lb, ub)
    ch = int(((lb > nl + 1e-7) | (ub < nu - 1e-7)).sum())
    print(f'周 {r}: 境界の変わった列 {ch}、固定 {fixed}、LP {lp_val(lb, ub, L):.4f}', flush=True)
    if ch == 0: break
LL, nc = coef(lb, ub)
print(f'係数の締め {nc} 個、LP {lp_val(lb, ub, LL):.4f}')
np.save(f.split('/')[-1] + '.bounds.npy', np.vstack([lb, ub]))
