"""連続列の部分が完全単模 (TU) になる列の集まりを、まとめて暗に整数とする判定の試し: python -I implint_tu.py <mps>
候補 = 境界が整数か無限の連続列。行ごとに、候補の係数の絶対値で割ると候補の係数が ±1、(暗に) 整数の列の係数と有限の左辺・右辺が整数、
候補でない連続列を含まない、を満たさない行の候補を外す。残りで、候補の列 (行への出現) が 2 個以下で、Heller-Tompkins の条件
(行を 2 組に分け、同じ符号の 2 つは別の組、逆の符号の 2 つは同じ組) を満たすように 2 色に塗れなければその列を外す。変化がなくなるまで。"""
import sys
from collections import deque
import highspy, numpy as np
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1])
lp = h.getLp(); n, m = lp.num_col_, lp.num_row_
A = lp.a_matrix_; st, idx, val = np.array(A.start_), np.array(A.index_), np.array(A.value_)
rl, ru, cl, cu = map(np.array, (lp.row_lower_, lp.row_upper_, lp.col_lower_, lp.col_upper_))
isint = [int(t) != 0 for t in lp.integrality_]
integral = lambda v: abs(v - round(v)) <= 1e-9 * max(abs(v), 1.0)
cols = [[(int(idx[k]), float(val[k])) for k in range(st[j], st[j+1])] for j in range(n)]
rows = [[] for _ in range(m)]
for j in range(n):
    for i, a in cols[j]:
        rows[i].append((j, a))
cand = [ (not isint[j]) and cl[j] < cu[j] and all((not np.isfinite(v)) or integral(v) for v in (cl[j], cu[j])) for j in range(n)]
fixed_cont = [ (not isint[j]) and cl[j] == cu[j] for j in range(n)]
print("候補", sum(cand))
while True:
    changed = False
    for i in range(m):
        cs = [(j, a) for j, a in rows[i] if cand[j]]
        if not cs:
            continue
        s = 1.0 / abs(cs[0][1])
        ok = all(abs(abs(a) * s - 1) <= 1e-9 for _, a in cs)
        ok = ok and all(isint[j] or cand[j] or (fixed_cont[j] and integral(cl[j] * a * s)) for j, a in rows[i])
        ok = ok and all(integral(a * s) for j, a in rows[i] if isint[j])
        ok = ok and all((not np.isfinite(v)) or integral(v * s) for v in (rl[i], ru[i]))
        if not ok:
            for j, _ in cs:
                cand[j] = False
            changed = True
    for j in range(n):
        if cand[j] and len(cols[j]) > 2:
            cand[j] = False; changed = True
    # Heller-Tompkins: 行の 2 色塗り。辺 = 候補の列 (行が 2 つ)。同符号なら色違い、逆符号なら同色
    color = {}
    adj = {}
    for j in range(n):
        if cand[j] and len(cols[j]) == 2:
            (i1, a1), (i2, a2) = cols[j]
            same = (a1 > 0) == (a2 > 0)
            adj.setdefault(i1, []).append((i2, same, j)); adj.setdefault(i2, []).append((i1, same, j))
    bad = set()
    for r0 in adj:
        if r0 in color:
            continue
        color[r0] = 0; q = deque([r0])
        while q:
            r = q.popleft()
            for r2, same, j in adj[r]:
                want = color[r] ^ (1 if same else 0)
                if r2 not in color:
                    color[r2] = want; q.append(r2)
                elif color[r2] != want:
                    bad.add(j)
    for j in bad:
        cand[j] = False; changed = True
    if not changed:
        break
if len(sys.argv) > 2:
    ids = [j for j in range(n) if cand[j]]
    # MPS の文字列に MARKER を差し込む (列名は C<番号>、列ごとに行が続く形を仮定)
    want = set(f"C{j}" for j in ids)
    out = []; inside = False; k = 0; incol = False
    for line in open(sys.argv[1]):
        t = line.split()
        if line.startswith("RHS"):
            if inside:
                out.append(f"    MX{k} 'MARKER' 'INTEND'\n"); inside = False
            incol = False
        if incol and t and t[0].startswith("C") and "'MARKER'" not in line:
            if t[0] in want and not inside:
                k += 1; out.append(f"    MX{k} 'MARKER' 'INTORG'\n"); inside = True
            elif t[0] not in want and inside:
                k += 1; out.append(f"    MX{k} 'MARKER' 'INTEND'\n"); inside = False
        out.append(line)
        if line.startswith("BOUNDS"):
            for j in ids:
                if not np.isfinite(cu[j]):
                    out.append(f" PL BND C{j}\n")
        if line.startswith("COLUMNS"):
            incol = True
    open(sys.argv[2], "w").writelines(out)
    print("整数にした列", len(ids))
print("TU の判定で暗に整数", sum(cand), "そのうち非零 2 の列", sum(1 for j in range(n) if cand[j] and len(cols[j]) == 2))
