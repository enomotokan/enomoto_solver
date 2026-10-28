"""多品種フローのネットワーク設計問題の乱数生成で、MIP の結果を HiGHS と比べる (MCF の cut-set カット `ENOMOTO_MIP_SEPA_MCF` の検査用)。

使い方: ENOMOTO_MIP_SEPA_MCF=1 python3 scripts/mip_fuzz_mcf.py 件数 [開始シード]
節点 8〜14 個、弧は乱数 (節点 0 から全節点へ届く木と、その逆向きを必ず入れる)、品種 1〜4 個 (品種ごとに始点 1 つから終点 1 つへ需要)。
弧の形は 3 通りを乱数で選ぶ:
- 向きあり: 流量 f_a^k (連続) と、弧の容量の単位数 y_a (0-1 か 0〜2 の整数)、`sum_k f_a^k <= cap_a y_a`
- 向きなし: 逆向きの 2 本の弧が容量を共有する `sum_k (f_ij^k + f_ji^k) <= cap y_ij`
- 2 値の流量: f_a^k は 0-1 で、容量の行は `sum_k d_k f_a^k <= cap_a y_a` (需要が係数)
流量保存は `out - in = 供給` (等式)。最小化の費用は流量の費用と固定費。
`ENOMOTO_MIP_DEBUG_MCF=1` を付けると、検出と分離のログ (標準エラー) で MCF のカットが出た問題の数を確かめられる。
"""
import random, sys, time
import highspy
from enomoto_solver import Model, Variable

inf = float("inf")


def gen(seed):
    g = random.Random(seed)
    nn = g.randint(8, 14)
    kind = g.choice(["dir", "undir", "bin"])
    arcs = set()
    for v in range(1, nn):
        u = g.randrange(v)
        arcs.add((u, v)); arcs.add((v, u))
    for _ in range(g.randint(nn, 2 * nn)):
        a, b = g.randrange(nn), g.randrange(nn)
        if a != b:
            arcs.add((a, b))
            if kind == "undir":
                arcs.add((b, a))
    arcs = sorted(arcs)
    nk = g.randint(1, 4)
    coms = []
    for _ in range(nk):
        s, t = g.sample(range(nn), 2)
        coms.append((s, t, g.randint(1, 6)))
    lo, up, c, it = [], [], [], []
    f = {}
    for e in arcs:
        cost = float(g.randint(0, 4))
        for k in range(nk):
            f[e, k] = len(lo); lo.append(0.0); c.append(cost)
            if kind == "bin":
                up.append(1.0); it.append(True)
            else:
                up.append(inf); it.append(False)
    rows, rlo, rup = [], [], []
    # 容量: 向きなしなら (i, j) と (j, i) で 1 本
    groups = []
    for e in arcs:
        if kind == "undir":
            if e[0] < e[1]:
                groups.append([e, (e[1], e[0])])
        else:
            groups.append([e])
    for grp in groups:
        cap = float(g.randint(3, 12))
        yu = 1.0 if g.random() < 0.6 else 2.0
        y = len(lo); lo.append(0.0); up.append(yu); c.append(float(g.randint(5, 40))); it.append(True)
        r = []
        for e in grp:
            for k in range(nk):
                r.append((f[e, k], float(coms[k][2]) if kind == "bin" else 1.0))
        r.append((y, -cap))
        rows.append(r); rlo.append(-inf); rup.append(0.0)
    for k, (s, t, d) in enumerate(coms):
        dd = 1.0 if kind == "bin" else float(d)
        for v in range(nn):
            r = []
            for e in arcs:
                if e[0] == v:
                    r.append((f[e, k], 1.0))
                if e[1] == v:
                    r.append((f[e, k], -1.0))
            b = dd if v == s else (-dd if v == t else 0.0)
            rows.append(r); rlo.append(b); rup.append(b)
    return lo, up, c, it, rows, rlo, rup


def highs(lp):
    lo, up, c, it, rows, rlo, rup = lp
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("mip_rel_gap", 1e-9)
    K = highspy.kHighsInf
    for j in range(len(lo)):
        h.addVar(lo[j], up[j] if up[j] < inf else K); h.changeColCost(j, c[j])
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
            print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
print("bad", bad, "of", N, stats, "time %.1f" % (time.time() - t0))
