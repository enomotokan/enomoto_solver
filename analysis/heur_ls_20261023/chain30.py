"""30n20b8: 違反した等式 (sum t y_t - s = 0) を満たすのに、時刻の整数列 s を上げたとき precedence の行 (s_a - s_b <= k) で
押し上げが要る列を辿る。python -I chain30.py <dump> <点> <違反行の番号> [HiGHS の解]"""
import sys, numpy as np, highspy, scipy.sparse as sp, collections
d, ptf, r0 = sys.argv[1], sys.argv[2], int(sys.argv[3])
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f"{d}/prob.mps"); lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
A = sp.csc_matrix((lp.a_matrix_.value_, lp.a_matrix_.index_, lp.a_matrix_.start_), shape=(m, n)); R = A.tocsr()
ridx = [int(s[1:]) for s in lp.row_names_]; rpos = {r: i for i, r in enumerate(ridx)}
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_); rlo, rup = np.array(lp.row_lower_), np.array(lp.row_upper_)
x = np.array([float(l) for l in open(ptf)]); act = R @ x
sol = np.array([float(l) for l in open(sys.argv[4])]) if len(sys.argv) > 4 else None
binc = (lo == 0) & (up == 1)
def row(i):
    s, e = R.indptr[i], R.indptr[i+1]; return list(zip(R.indices[s:e], R.data[s:e]))
def col(j):
    s, e = A.indptr[j], A.indptr[j+1]; return list(zip(A.indices[s:e], A.data[s:e]))
# 時刻の整数列ごとの定義の等式 (sum t y_t - s = 0) と、許される時刻
defrow = {}
for i in range(m):
    t = row(i)
    if rlo[i] == rup[i] == 0:
        ints = [(j, a) for j, a in t if not binc[j] and lo[j] < up[j]]
        if len(ints) == 1 and ints[0][1] == -1:
            j = ints[0][0]
            allowed = sorted(a for jj, a in t if binc[jj])
            defrow.setdefault(j, []).append((i, allowed))
prec = collections.defaultdict(list)  # a -> [(b, k)] で s_a - s_b <= k
for i in range(m):
    t = row(i)
    if len(t) == 2 and not binc[t[0][0]] and not binc[t[1][0]] and np.isinf(rlo[i]):
        (j1, a1), (j2, a2) = t
        if a1 == 1 and a2 == -1: prec[j1].append((j2, rup[i]))
        if a1 == -1 and a2 == 1: prec[j2].append((j1, rup[i]))
print(f"時刻の整数列 (定義の等式あり) {len(defrow)}、precedence の行 {sum(len(v) for v in prec.values())}")
i0 = rpos[r0]
s0 = [j for j, a in row(i0) if not binc[j] and lo[j] < up[j]][0]
cur = act[i0] + x[s0]  # 選ばれた時刻
print(f"違反行 r{r0}: 時刻列 c{s0} = {x[s0]:g}、選ばれた時刻 (sum t y_t) = {cur:g}、許される時刻 {[a for a in defrow[s0][0][1]][:10]}...")
# 押し上げ: s0 を cur にし、prec で下流を必要なだけ上げる。上げた列の時刻が許されない値なら、次に許される時刻まで上げる
need = {s0: cur}
queue = [s0]; changed = {}
while queue:
    a = queue.pop()
    va = need[a]
    for b, k in prec[a]:
        req = va - k
        cb = need.get(b, x[b])
        if req > cb + 1e-9:
            # 許される時刻へ
            if b in defrow:
                al = [t for t in defrow[b][0][1] if t >= req]
                req = al[0] if al else float('inf')
            need[b] = req
            queue.append(b)
for j in need:
    if j != s0:
        print(f"  c{j}: {x[j]:g} -> {need[j]:g} (上限 {up[j]:g})、定義の等式 {len(defrow.get(j, []))} 本")
print(f"押し上げが要る時刻列: {len(need) - 1} 本 (+ c{s0})。各列で 0-1 の入れ替え 2 本 + 定義の等式の時刻列がもう 1 本ずつ")
# 上流 (s0 を上げると壊れる行の向き) も数える: prec で s0 の上限になっているもの
if sol is not None:
    print(f"HiGHS の解: c{s0} = {sol[s0]:g}、" + ", ".join(f"c{j}={sol[j]:g}" for j in need if j != s0))
    tim = [j for j in defrow]
    diff = [j for j in tim if abs(sol[j] - x[j]) > 0.5]
    print(f"時刻列 {len(tim)} 本のうち FJ の点と HiGHS の解で値の違う列 {len(diff)}、0-1 列で違う列 {int(((np.abs(sol - x) > 0.5) & binc).sum())}")
