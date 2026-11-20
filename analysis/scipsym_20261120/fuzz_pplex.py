"""試作 ENOMOTO_MIP_SYM_PPLEX の誤答の確かめ: 詰め込みの行に入る対合をもつ対称な問題を作り、HiGHS と状態・最適値を突き合わせ、
当方の解の実行可能性と目的値も確かめる。python fuzz_pplex.py 問題数 [開始seed]
型 (seed で選ぶ):
 0. Z_2^k (k = 3〜6) のケーリーグラフの最大独立集合 (辺 x_u + x_v <= 1)。生成集合 D はランダム。頂点の番号は乱数で並べ替える。
    一部で重み・目的を対称なまま (全て -1) か、軌道ごとに同じ重みにする (Hamming の重さで重み)。一部で総数の上限 sum x <= K。
 1. 巡回グラフ Z_n (n = 6〜40) の最大独立集合 (辺 x_u + x_v <= 1、または 3 点のクリーク行)。一部で 1 列の境界を変えて対称性を崩す。
 2. 同じ機械への割り当て (行 sum_m x_{i,m} <= 1 か = 1、容量の行)。オービトープを切って (ENOMOTO_MIP_NO_ORBITOPE=1 を外から) 回すと PPLEX に回る。"""
import random, sys, time, highspy
from enomoto_solver import Model, Variable

inf = float('inf')


def gen(seed):
    g = random.Random(seed)
    typ = seed % 3
    lo, up, c, it, rows, rlo, rup = [], [], [], [], [], [], []
    if typ in (0, 1):
        if typ == 0:
            k = g.randint(3, 7); n = 1 << k
            if g.random() < 0.6:  # 符号の型: Hamming の距離が集合 W に入る組を辺に (座標の置換と平行移動の対称性)
                W = set(range(1, g.randint(2, min(4, k)) + 1)) if g.random() < 0.6 else set(g.sample(range(1, k + 1), g.randint(1, max(1, k // 2))))
                D = {d for d in range(1, n) if bin(d).count('1') in W}
            else:
                D = set()
                while len(D) < g.randint(1, max(1, k)):
                    D.add(g.randrange(1, n))
            edges = {(min(u, u ^ d), max(u, u ^ d)) for u in range(n) for d in D}
            wkind = g.randrange(3)
            w = [-1.0 if wkind == 0 else -float(1 + bin(u).count('1') % 3) if wkind == 1 else -float(g.randint(1, 3)) for u in range(n)]
        else:
            n = g.randint(6, 40)
            S = g.sample(range(1, n // 2 + 1), g.randint(1, min(3, n // 2)))
            edges = {(min(u, (u + s) % n), max(u, (u + s) % n)) for u in range(n) for s in S}
            w = [-1.0] * n
        perm = list(range(n)); g.shuffle(perm)  # 頂点 u は列 perm[u]
        for u in range(n):
            lo.append(0.0); up.append(1.0); c.append(0.0); it.append(True)
        for u in range(n):
            c[perm[u]] = w[u]
        if typ == 1 and g.random() < 0.2:
            up[g.randrange(n)] = 0.0
        for (a, b) in sorted(edges):
            rows.append([(perm[a], 1.0), (perm[b], 1.0)]); rlo.append(-inf); rup.append(1.0)
        if g.random() < 0.3:
            rows.append([(j, 1.0) for j in range(n)]); rlo.append(-inf); rup.append(float(g.randint(1, max(1, n // 4))))
        if g.random() < 0.15:  # 実行不能になりうる下限
            rows.append([(j, 1.0) for j in range(n)]); rlo.append(float(g.randint(n // 4, n // 2 + 1))); rup.append(inf)
    else:
        ni = g.randint(3, 10); nm = g.randint(2, 5)
        w = [g.randint(1, 9) for _ in range(ni)]; p = [g.randint(1, 20) for _ in range(ni)]
        cap = g.randint(5, 20); exact = g.random() < 0.5
        xid = {}
        for i in range(ni):
            for m in range(nm):
                xid[i, m] = len(lo); lo.append(0.0); up.append(1.0); c.append(-float(p[i])); it.append(True)
        for i in range(ni):
            rows.append([(xid[i, m], 1.0) for m in range(nm)]); rlo.append(1.0 if exact else -inf); rup.append(1.0)
        for m in range(nm):
            rows.append([(xid[i, m], float(w[i])) for i in range(ni)]); rlo.append(-inf); rup.append(float(cap))
    return lo, up, c, it, rows, rlo, rup


def highs(lp):
    lo, up, c, it, rows, rlo, rup = lp
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.setOptionValue('mip_rel_gap', 1e-9)
    K = highspy.kHighsInf
    for j in range(len(lo)):
        h.addVar(lo[j], up[j]); h.changeColCost(j, c[j])
        if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in zip(rows, rlo, rup):
        h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
    h.run(); return h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value


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
    s = M.solve(mip_rel_gap=1e-9)
    return s.status, s.objective, [x.value for x in xs] if s.status == "optimal" else None


def feasible(lp, x):
    lo, up, c, it, rows, rlo, rup = lp
    if any(v < lo[j] - 1e-6 or v > up[j] + 1e-6 or (it[j] and abs(v - round(v)) > 1e-6) for j, v in enumerate(x)): return False
    for r, l, u in zip(rows, rlo, rup):
        a = sum(v * x[j] for j, v in r)
        if a < l - 1e-6 or a > u + 1e-6: return False
    return True


N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed)
    hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
    ok = (hmap == os_) and (hmap != 'optimal' or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if ok and x is not None:
        ok = feasible(lp, x) and abs(sum(cc * v for cc, v in zip(lp[2], x)) - oo) <= 1e-6 * (1 + abs(oo))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0))
