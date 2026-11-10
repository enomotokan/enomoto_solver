"""PAPER7 の誤答の検査: TU の部分による暗黙の整数 (ENOMOTO_MIP_IMPLINT_TU2) が効く形の小さな MIP を乱数で作り、HiGHS と最適値を突き合わせる。
形: (A) 固定費用の流れ (保存の行・x <= u y)、(B) 容量つきのロットサイズ (在庫の流れ・端数の在庫の上限)、(C) 開始時刻の差の制約とメイクスパン
(目的値の刻み)、(D) 2 品種の流れと共同の容量。それぞれに確率で TU を壊す要素 (端数の右辺・端数の係数・奇数の閉路・端数の境界) を混ぜる。
判定: 状態が同じで最適値が相対 1e-6 で一致、ENOMOTO の点が元の問題で許容誤差 1e-6 で実行可能で目的値が一致。
TU2 の印が何本付いたかを、ログ (ENOMOTO_MIP_LOG=1 の「with the TU2 rule」の行、fd 2 を一時ファイルに向けて読む) から数える。
使い方: ENOMOTO_MIP_IMPLINT_TU2=bas python fuzz_tu.py 問題数 [開始seed]   (TU2 の値は環境変数のまま。付けなければ既定の動きの検査)"""
import os, random, sys, tempfile, time, re
os.environ.setdefault("ENOMOTO_MIP_LOG", "1")
import highspy
from enomoto_solver import Model, Variable
inf = float("inf")


def gen(seed):
    g = random.Random(seed)
    kind = g.choice("ABCD")
    lo, up, c, it, rows, rlo, rup = [], [], [], [], [], [], []
    def var(l, u, cost, integer):
        lo.append(l); up.append(u); c.append(cost); it.append(integer); return len(lo) - 1
    def row(r, l, u):
        rows.append(r); rlo.append(l); rup.append(u)
    brk = g.random() < 0.5  # TU を壊す要素を混ぜる問題か (混ぜるなら要素ごとに 3 割)
    frac = lambda: brk and g.random() < 0.3
    if kind in "AD":
        nv = g.randint(3, 7); K = 2 if kind == "D" else 1
        arcs = {(a, b) for _ in range(g.randint(nv, 3 * nv)) for a, b in [g.sample(range(nv), 2)]}
        sup = [0] * nv
        for _ in range(g.randint(1, 3)):
            s, t = g.sample(range(nv), 2); d = g.randint(1, 9); sup[s] += d; sup[t] -= d
            arcs.add((s, t))  # 実行可能にするため直接の枝を足す
        arcs = sorted(arcs)
        y = {a: var(0, 1, g.randint(1, 30), True) for a in arcs}
        x = {(a, k): var(0, inf if g.random() < 0.5 else g.randint(10, 30), round(g.uniform(0, 5), g.choice([0, 1, 2])), False) for a in arcs for k in range(K)}
        for k in range(K):
            for v in range(nv):
                r = [(x[(a, k)], 1.0) for a in arcs if a[0] == v] + [(x[(a, k)], -1.0) for a in arcs if a[1] == v]
                if r:
                    b = float(sup[v]) + (0.5 if frac() and g.random() < 0.3 else 0.0)
                    kk = g.randrange(4)
                    if kk <= 1: row(r, b, b)
                    elif kk == 2: row(r, -inf, b + 2)
                    else: row(r, b - 2, inf)
        for a in arcs:
            u = g.randint(10, 30) if not frac() else g.randint(10, 30) + 0.5
            row([(x[(a, k)], 1.0) for k in range(K)] + [(y[a], -float(u))], -inf, 0.0)
        if frac():
            a, b = g.sample(arcs, 2) if len(arcs) > 1 else (arcs[0], arcs[0])
            row([(x[(a, 0)], 1.0), (x[(b, 0)], g.choice([1.0, 1.5, 2.0]))], -inf, float(g.randint(3, 15)))
        if frac():  # 奇数の閉路
            i1, i2, i3 = [x[(g.choice(arcs), 0)] for _ in range(3)]
            if len({i1, i2, i3}) == 3:
                for p, q in [(i1, i2), (i2, i3), (i1, i3)]:
                    row([(p, 1.0), (q, 1.0)], -inf, float(g.randint(1, 9)))
    elif kind == "B":
        T = g.randint(3, 8); P = g.randint(1, 3)
        xs = {}; ss = {}
        for p in range(P):
            prev = None
            for t in range(T):
                yv = var(0, 1, g.randint(5, 40), True)
                xv = var(0, inf, round(g.uniform(0, 3), g.choice([0, 1, 2])), False)
                sv = var(0, inf if g.random() < 0.6 else g.randint(5, 30) + (0.5 if frac() else 0.0), round(g.uniform(0, 2), g.choice([0, 1])), False)
                d = g.randint(0, 9)
                r = [(xv, 1.0), (sv, -1.0)] + ([(prev, 1.0)] + [] if prev is not None else [])
                row(r, float(d), float(d))
                M = g.randint(20, 60)
                row([(xv, 1.0), (yv, -float(M) - (0.5 if frac() else 0.0))], -inf, 0.0)
                xs[(p, t)] = xv; prev = sv
        for t in range(T):
            co = [1.0] * P if not frac() else [g.choice([1.0, 2.0, 1.5]) for _ in range(P)]
            row([(xs[(p, t)], co[p]) for p in range(P)], -inf, float(g.randint(10 * P, 40 * P)))
    else:  # C
        J = g.randint(3, 6)
        S = [var(0, inf, 0.0, False) for _ in range(J)]
        Cm = var(0, inf, 1.0, False)
        pt = [g.randint(1, 9) for _ in range(J)]
        for j in range(J):
            row([(Cm, 1.0), (S[j], -1.0)], float(pt[j]) + (0.5 if frac() else 0.0), inf)
        for i in range(J):
            for j in range(i + 1, J):
                if g.random() < 0.7:
                    yv = var(0, 1, 0.0, True); M = 100.0
                    row([(S[j], 1.0), (S[i], -1.0), (yv, -M)], float(pt[i]) - M, inf)
                    row([(S[i], 1.0), (S[j], -1.0), (yv, M)], float(pt[j]), inf)
        if frac():
            row([(S[0], 1.0), (S[1 % J], 1.0), (S[2 % J], 1.0)], -inf, float(g.randint(60, 90)))
        for j in range(J):
            if g.random() < 0.3:
                row([(S[j], 1.0)], float(g.randint(0, 5)) + (0.5 if frac() else 0.0), inf)
    # 費用は非負、列は下限 0 なので有界 (全部の列の和の行は TU を壊すので置かない)
    return kind, (lo, up, c, it, rows, rlo, rup)


def highs(lp):
    lo, up, c, it, rows, rlo, rup = lp
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("mip_rel_gap", 1e-9)
    K = highspy.kHighsInf
    for j in range(len(lo)):
        h.addVar(lo[j], up[j] if up[j] < inf else K); h.changeColCost(j, c[j])
        if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in zip(rows, rlo, rup):
        h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
    h.run()
    return h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value


def ours(lp):
    lo, up, c, it, rows, rlo, rup = lp
    M = Model(); xs = [Variable(int if it[j] else float, lo[j], up[j]) for j in range(len(lo))]
    M.set_objective(sum(cc * x for cc, x in zip(c, xs)))
    for r, l, u in zip(rows, rlo, rup):
        e = sum(v * xs[j] for j, v in r)
        if l == u: M.add_constraint(e == l)
        else:
            if l > -inf: M.add_constraint(e >= l)
            if u < inf: M.add_constraint(e <= u)
    sys.stdout.flush()
    fd, path = tempfile.mkstemp(); saved = os.dup(2); saved1 = os.dup(1); os.dup2(fd, 2); os.dup2(fd, 1)
    try:
        s = M.solve(mip_rel_gap=1e-9)
    finally:
        sys.stdout.flush(); os.dup2(saved, 2); os.dup2(saved1, 1); os.close(saved); os.close(saved1); os.close(fd)
    log = open(path).read(); os.unlink(path)
    m = re.findall(r"\+(\d+) with the TU2 rule", log)
    return s.status, s.objective, ([x.value for x in xs] if s.status == "optimal" else None), (int(m[0]) if m else 0)


def feasible(lp, x, tol=1e-6):
    lo, up, c, it, rows, rlo, rup = lp
    for j in range(len(lo)):
        if x[j] < lo[j] - tol or x[j] > up[j] + tol or (it[j] and abs(x[j] - round(x[j])) > tol): return False
    for r, l, u in zip(rows, rlo, rup):
        a = sum(v * x[j] for j, v in r)
        if a < l - tol * (1 + abs(l)) or a > u + tol * (1 + abs(u)): return False
    return True


if __name__ == "__main__":
    N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    bad = 0; marked = 0; t0 = time.time(); kinds = {}
    for seed in range(start, start + N):
        kind, lp = gen(seed)
        hs, ho = highs(lp); os_, oo, x, nm = ours(lp)
        hmap = {"Optimal": "optimal", "Infeasible": "infeasible", "Unbounded": "unbounded"}.get(hs, hs)
        ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
        if ok and x is not None:
            ok = feasible(lp, x) and abs(sum(cc * v for cc, v in zip(lp[2], x)) - oo) <= 1e-6 * (1 + abs(oo))
        marked += nm > 0
        st = kinds.setdefault("status", {}); st[os_] = st.get(os_, 0) + 1
        kinds.setdefault(kind, [0, 0, 0])
        kinds[kind][0] += 1; kinds[kind][1] += nm > 0; kinds[kind][2] += not ok
        if not ok:
            bad += 1
            if bad <= 10: print("seed", seed, kind, "highs", hs, ho, "ours", os_, oo, "tu2", nm, flush=True)
    print("bad", bad, "of", N, "TU2 marked in", marked, "kinds", kinds, "time %.1f" % (time.time() - t0))
