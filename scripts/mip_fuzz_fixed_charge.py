"""固定費ネットワークフロー問題の乱数生成で、MIP の結果を HiGHS と比べる (VUB 置き換え・経路集約の検査用)。

使い方: python3 scripts/mip_fuzz_fixed_charge.py 件数 [開始シード]
各辺 (u, v) に連続のフロー x (容量 cap) と、固定費の 0-1 変数 y (x <= cap * y) を持たせる
(一部の辺は固定費なし)。節点ごとに流入 - 流出 = 需要 (供給は負)。
"""
import random, sys, time
import highspy
from enomoto_solver import Model, Variable

inf = float("inf")


def gen(seed):
    g = random.Random(seed)
    nn = g.randint(3, 8)
    arcs = set()
    for _ in range(g.randint(nn, 3 * nn)):
        a, b = g.randrange(nn), g.randrange(nn)
        if a != b:
            arcs.add((a, b))
    # 節点 0 から全節点へ届く木を必ず入れる (実行不能な例ばかりにならないように)
    for v in range(1, nn):
        arcs.add((g.randrange(v), v))
    arcs = sorted(arcs)
    dem = [g.randint(-3, 5) for _ in range(nn)]
    dem[0] = -sum(dem[1:]) - g.randint(0, 3)  # 節点 0 が供給源 (余りは捨て口なしなので <= で扱う)
    # 変数: 各辺に x (連続) と y (0-1、固定費あり辺のみ)
    lo, up, c, it = [], [], [], []
    xi, yi = {}, {}
    for e in arcs:
        xi[e] = len(lo); lo.append(0.0); up.append(float(g.randint(3, 15))); c.append(float(g.randint(0, 5))); it.append(False)
    for e in arcs:
        if g.random() < 0.8:
            yi[e] = len(lo); lo.append(0.0); up.append(1.0); c.append(float(g.randint(1, 20))); it.append(True)
    rows, rlo, rup = [], [], []
    for e, j in xi.items():
        if e in yi:
            rows.append([(j, 1.0), (yi[e], -up[j])]); rlo.append(-inf); rup.append(0.0)
    for v in range(nn):
        r = []
        for e, j in xi.items():
            if e[1] == v:
                r.append((j, 1.0))
            if e[0] == v:
                r.append((j, -1.0))
        if not r:
            continue
        rows.append(r)
        if v == 0:
            rlo.append(dem[0]); rup.append(inf)  # 供給源は使い切らなくてよい
        else:
            rlo.append(float(dem[v])); rup.append(float(dem[v]))
    return lo, up, c, it, rows, rlo, rup


def highs(lp):
    lo, up, c, it, rows, rlo, rup = lp
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("mip_rel_gap", 1e-9)
    K = highspy.kHighsInf
    for j in range(len(lo)):
        h.addVar(lo[j], up[j]); h.changeColCost(j, c[j])
        if it[j]:
            h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in zip(rows, rlo, rup):
        h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
    h.run()
    return h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value


def ours(lp):
    lo, up, c, it, rows, rlo, rup = lp
    M = Model(); xs = [Variable(int if it[j] else float, lo[j], up[j]) for j in range(len(lo))]
    M.set_objective(sum((cc * x for cc, x in zip(c, xs)), 0.0 * xs[0]))
    for r, l, u in zip(rows, rlo, rup):
        e = sum(v * xs[j] for j, v in r)
        if l == u:
            M.add_constraint(e == l)
        else:
            if l > -inf:
                M.add_constraint(e >= l)
            if u < inf:
                M.add_constraint(e <= u)
    s = M.solve(mip_rel_gap=1e-9)
    return s.status, s.objective


N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time(); stats = {}
for seed in range(start, start + N):
    lp = gen(seed)
    hs, ho = highs(lp); os_, oo = ours(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible", "Unbounded": "unbounded"}.get(hs, hs)
    stats[hmap] = stats.get(hmap, 0) + 1
    ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10:
            print("seed", seed, "highs", hs, ho, "ours", os_, oo)
print("bad", bad, "of", N, stats, "time %.1f" % (time.time() - t0))
