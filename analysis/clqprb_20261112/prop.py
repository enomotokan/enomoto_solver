"""CLQPRB: 行の活動による境界の伝播 (ENOMOTO の Domain::propagate と同じ種類) とクリーク表による伝播を Python で試す小さな道具。
使い方: python prop.py 問題 列番号...  → 列ごとに x_j = 1 を伝播して矛盾するか (行だけ / 行 + 行から取り出したクリーク)"""
import sys, json
import numpy as np
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent; WT = D.parents[1]
INF = 1e30
def load(p):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(WT / ".miplib_cache" / f"{p}.mps"))
    lp = h.getLp(); A = lp.a_matrix_; m, n = lp.num_row_, lp.num_col_
    st, ix, va = list(A.start_), list(A.index_), list(A.value_)
    rows = [[] for _ in range(m)]; cols = [[] for _ in range(n)]
    for j in range(n):
        for k in range(st[j], st[j + 1]):
            rows[ix[k]].append((j, va[k])); cols[j].append((ix[k], va[k]))
    isint = [str(t).endswith("kInteger") for t in lp.integrality_] if lp.integrality_ else [False] * n
    return dict(m=m, n=n, rows=rows, cols=cols, rl=list(lp.row_lower_), ru=list(lp.row_upper_), lo=list(lp.col_lower_), up=list(lp.col_upper_), isint=isint, names=list(lp.col_names_))
def propagate(P, lo, up, queue_rows=None, clq=None, maxit=10**7):
    """行の活動で境界を締める。clq: 文字 (2j + v、v=1 なら x_j=1) -> 衝突する文字の集合。矛盾なら False"""
    rows, cols, rl, ru, isint = P["rows"], P["cols"], P["rl"], P["ru"], P["isint"]
    q = list(range(P["m"])) if queue_rows is None else list(queue_rows); inq = set(q)
    fixedq = [j for j in range(P["n"]) if lo[j] == up[j]] if clq else []
    it = 0
    while q or fixedq:
        while fixedq and clq is not None:
            j = fixedq.pop()
            lit = 2 * j + (1 if lo[j] >= 0.5 else 0)
            for o in clq.get(lit, ()):
                k, v = o >> 1, o & 1
                # 文字 o (x_k = v) は lit と衝突: x_k != v
                if v == 1:
                    if lo[k] > 0.5: return False
                    if up[k] > 0: up[k] = 0.0; fixedq.append(k); [ (q.append(i), inq.add(i)) for i, _ in cols[k] if i not in inq]
                else:
                    if up[k] < 0.5: return False
                    if lo[k] < 1: lo[k] = 1.0; fixedq.append(k); [ (q.append(i), inq.add(i)) for i, _ in cols[k] if i not in inq]
        if not q: break
        i = q.pop(); inq.discard(i); it += 1
        if it > maxit: break
        r = rows[i]
        mn = sum(a * (lo[j] if a > 0 else up[j]) for j, a in r)
        mx = sum(a * (up[j] if a > 0 else lo[j]) for j, a in r)
        if mn > ru[i] + 1e-9 or mx < rl[i] - 1e-9: return False
        for j, a in r:
            if lo[j] == up[j]: continue
            # 上側: a x_j <= ru - (mn - 寄与)
            cmin = a * (lo[j] if a > 0 else up[j]); cmax = a * (up[j] if a > 0 else lo[j])
            nl, nu = lo[j], up[j]
            if ru[i] < INF and mn > -INF:
                rest = mn - cmin
                b = (ru[i] - rest) / a
                if a > 0: nu = min(nu, b)
                else: nl = max(nl, b)
            if rl[i] > -INF and mx < INF:
                rest = mx - cmax
                b = (rl[i] - rest) / a
                if a > 0: nl = max(nl, b)
                else: nu = min(nu, b)
            if isint[j]:
                nl = np.ceil(nl - 1e-6); nu = np.floor(nu + 1e-6)
            if nl > nu + 1e-9: return False
            if nl > lo[j] + 1e-9 or nu < up[j] - 1e-9:
                lo[j], up[j] = max(lo[j], nl), min(up[j], nu)
                if lo[j] == up[j] and clq is not None: fixedq.append(j)
                for i2, _ in cols[j]:
                    if i2 not in inq: q.append(i2); inq.add(i2)
    return True
def row_cliques(P):
    """係数・境界から 0-1 列 2 本が同時に取れない組 (衝突の組) を行ごとに取り出す (長い行は 2 番目に大きい寄与の組だけ見る簡易版)"""
    lo, up, isint = P["lo"], P["up"], P["isint"]
    binj = [isint[j] and lo[j] == 0 and up[j] == 1 for j in range(P["n"])]
    clq = {}
    def add(a, b):
        clq.setdefault(a, set()).add(b); clq.setdefault(b, set()).add(a)
    for i, r in enumerate(P["rows"]):
        for sgn, rhs in ((1, P["ru"][i]), (-1, -P["rl"][i])):
            if rhs >= INF / 2: continue
            # sgn * a x <= rhs。0-1 列を「寄与の増え」が正になる向きの文字に直す
            base = 0.0; lits = []; ok = True
            for j, a in r:
                a = sgn * a
                if binj[j]:
                    if a > 0: lits.append((a, 2 * j + 1))
                    else: base += a; lits.append((-a, 2 * j))
                else:
                    if a > 0:
                        if lo[j] <= -INF / 2: ok = False; break
                        base += a * lo[j]
                    else:
                        if up[j] >= INF / 2: ok = False; break
                        base += a * up[j]
            if not ok: continue
            slack = rhs - base
            big = sorted([t for t in lits if t[0] > slack / 2 + 1e-9], reverse=True)
            for x in range(len(big)):
                for y in range(x + 1, len(big)):
                    if big[x][0] + big[y][0] > slack + 1e-9: add(big[x][1], big[y][1])
                    else: break
    return clq
if __name__ == "__main__":
    P = load(sys.argv[1])
    clq = row_cliques(P)
    print("clique literals", len(clq), "edges", sum(len(v) for v in clq.values()) // 2, file=sys.stderr)
    for s in sys.argv[2:]:
        j = int(s)
        lo, up = P["lo"][:], P["up"][:]; lo[j] = 1.0
        r1 = propagate(P, lo, up)
        lo, up = P["lo"][:], P["up"][:]; lo[j] = 1.0
        r2 = propagate(P, lo, up, clq=clq)
        print(json.dumps({"col": j, "rows_ok": r1, "rows_clique_ok": r2}))
