"""FJ の書き出した点 (ENOMOTO_MIP_FJ_DUMP) の違反行を調べる。
python -I viol_rows.py <dumpディレクトリ> <点のファイル> [HiGHS の解 (.sol.txt、1 行 1 値)]

- 違反行ごとに: 向き・右辺・左辺・長さ、列の種類 (整数/連続、境界、値) と係数の分布
- その行を 1 列だけ動かして満たす手 (列ごと): 動かす量、動かしたときに新たに違反する行の数 (重みなし)
- 2 列を同時に動かす手で、新たな違反を出さずに満たせる組があるか (違反行の列の組だけ、上限つき)
- HiGHS の解との違い (整数列の値が違う列の数、違反行の列のうち違う列)
"""
import sys, collections
import numpy as np
import highspy

d, ptf = sys.argv[1], sys.argv[2]
h = highspy.Highs()
h.setOptionValue("output_flag", False)
h.readModel(f"{d}/prob.mps")
lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
A = lp.a_matrix_
# 列方向 (HiGHS の既定は列方向)
start, index, value = np.array(A.start_), np.array(A.index_), np.array(A.value_)
names = list(lp.col_names_)
# write_mps は列を c{j} で書くので、名前から元の番号を取る
cidx = [int(s[1:]) for s in names]
assert cidx == list(range(n)), "列の並びが違う"
rnames = list(lp.row_names_)
ridx = [int(s[1:]) for s in rnames]
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
rlo, rup = np.array(lp.row_lower_), np.array(lp.row_upper_)
integ = np.array([int(t) == 1 for t in lp.integrality_]) if len(lp.integrality_) else np.zeros(n, bool)
cost = np.array(lp.col_cost_)
import scipy.sparse as sp
Acsc = sp.csc_matrix((value, index, start), shape=(m, n))
Acsr = Acsc.tocsr()
x = np.array([float(l) for l in open(ptf)])
act = Acsr @ x
tol = 1e-6


def viol(i, a):
    t = tol * (1 + min(abs(a), 1e6))
    if a > rup[i] + t:
        return a - rup[i]
    if a < rlo[i] - t:
        return rlo[i] - a
    return 0.0


V = [i for i in range(m) if viol(i, act[i]) > 0]
print(f"問題: 列 {n} (整数 {integ.sum()}、うち 0-1 {(integ & (lo == 0) & (up == 1)).sum()})、行 {m} (等式 {(rlo == rup).sum()})")
print(f"点: 違反行 {len(V)}、違反量 {sum(viol(i, act[i]) for i in V):.4g}、目的 {cost @ x:.6g}")
print(f"列の範囲外: {int(((x < lo - 1e-9) | (x > up + 1e-9)).sum())}、整数でない整数列: {int((integ & (np.abs(x - np.round(x)) > 1e-9)).sum())}")


def coltype(j):
    if integ[j] and lo[j] == 0 and up[j] == 1:
        return "bin"
    return ("int" if integ[j] else "cont") + f"[{lo[j]:g},{up[j]:g}]"


def rowtype(i):
    return "=" if rlo[i] == rup[i] else ("<=" if np.isinf(rlo[i]) else (">=" if np.isinf(rup[i]) else "rng"))


def rowdesc(i):
    s, e = Acsr.indptr[i], Acsr.indptr[i + 1]
    cols, a = Acsr.indices[s:e], Acsr.data[s:e]
    kinds = collections.Counter(coltype(j) for j in cols)
    coefs = collections.Counter(f"{v:g}" for v in a)
    return cols, a, kinds, coefs


newviol_cache = {}


def single_moves(i):
    """行 i を 1 列だけ動かして満たす手: (列, 新しい値, 新たに違反する行の数, 新たな違反量)"""
    cols, a, _, _ = rowdesc(i)
    need_lo, need_up = rlo[i] - act[i], rup[i] - act[i]  # 左辺の変化の範囲
    out = []
    for j, aj in zip(cols, a):
        # aj * d in [need_lo, need_up]
        dl, du = sorted([need_lo / aj, need_up / aj]) if True else None
        if integ[j]:
            dl, du = np.ceil(dl - 1e-9), np.floor(du + 1e-9)
        if dl > du:
            continue
        dd = 0.0 if dl <= 0 <= du else (dl if dl > 0 else du)
        nv = x[j] + dd
        if nv < lo[j] - 1e-9 or nv > up[j] + 1e-9:
            # 境界で切る
            nv = min(max(nv, lo[j]), up[j])
            dd = nv - x[j]
            if not (need_lo - 1e-9 <= aj * dd <= need_up + 1e-9):
                continue
        s, e = Acsc.indptr[j], Acsc.indptr[j + 1]
        nv_rows, nv_amt = 0, 0.0
        for r, b in zip(Acsc.indices[s:e], Acsc.data[s:e]):
            if r == i:
                continue
            before, after = viol(r, act[r]), viol(r, act[r] + b * dd)
            if after > before:
                nv_rows += 1
                nv_amt += after - before
        out.append((j, nv, nv_rows, nv_amt))
    return out


sol = None
if len(sys.argv) > 3:
    sol = np.array([float(l) for l in open(sys.argv[3])])
    dif = np.where(np.abs(sol - x) > 1e-6)[0]
    print(f"HiGHS の解 (目的 {cost @ sol:.6g}) との違い: 値の違う列 {len(dif)} (整数列 {integ[dif].sum()})")

for i in V:
    cols, a, kinds, coefs = rowdesc(i)
    print(f"\n== 行 r{ridx[i]} ({rowtype(i)}) 下限 {rlo[i]:g} 上限 {rup[i]:g} 左辺 {act[i]:.6g} 違反 {viol(i, act[i]):.4g} 長さ {len(cols)}")
    print(f"   列の種類: {dict(kinds)}")
    print(f"   係数: {dict(coefs.most_common(8))}")
    vals = collections.Counter(f"{coltype(j)}={x[j]:g}" for j in cols)
    print(f"   列の値: {dict(vals.most_common(10))}")
    mv = single_moves(i)
    print(f"   1 列で満たす手: {len(mv)} 個。新たな違反の行数の分布: {dict(collections.Counter(r for _, _, r, _ in mv))}")
    for j, nv, r, amt in sorted(mv, key=lambda t: (t[2], t[3]))[:5]:
        s, e = Acsc.indptr[j], Acsc.indptr[j + 1]
        broken = [(int(rr), b) for rr, b in zip(Acsc.indices[s:e], Acsc.data[s:e]) if rr != i and viol(rr, act[rr] + b * (nv - x[j])) > viol(rr, act[rr])]
        print(f"     c{j} {coltype(j)} {x[j]:g} -> {nv:g}: 新たな違反 {r} 行 ({amt:.4g}) " + "; ".join(f"r{ridx[rr]}({rowtype(rr)},len {Acsr.indptr[rr+1]-Acsr.indptr[rr]},rhs {rlo[rr]:g}/{rup[rr]:g},act {act[rr]:g},coef {b:g})" for rr, b in broken[:4]))
    if sol is not None:
        dd = [j for j in cols if abs(sol[j] - x[j]) > 1e-6]
        print(f"   HiGHS の解でこの行の列のうち値の違う列: {len(dd)}: " + ", ".join(f"c{j} {x[j]:g}->{sol[j]:g}" for j in dd[:12]))
