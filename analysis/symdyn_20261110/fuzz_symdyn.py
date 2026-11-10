"""試作 ENOMOTO_MIP_SYMDYN・ENOMOTO_MIP_SYMREFL の正しさ確認用: 対称性をもつ乱数の問題を HiGHS の最適値と突き合わせる。
python fuzz_symdyn.py 問題数 [開始seed] [大きさの倍率]
種類: 頂点推移的なグラフ (巡回グラフ・超立方体) 上の安定集合・頂点被覆・支配集合 (置換)、同じ機械への割り当て (置換、
連続列の負荷つき)、同じ品の一般整数のナップサック (置換、一般整数)、重み付き max-cut・グラフの二等分・max-k-cut の 2 値版
(反転: x → 1 − x)、一般整数の反転 (x ∈ [0, U]、|x_i − x_j| を補助列で表す)、反転と置換の組み合わせ (巡回グラフ上の max-cut)。"""
import random, sys, time, highspy
from enomoto_solver import Model, Variable

inf = float('inf')
SCALE = int(sys.argv[3]) if len(sys.argv) > 3 else 1


def circulant(n, jumps):
    e = set()
    for i in range(n):
        for d in jumps:
            a, b = i, (i + d) % n
            if a != b:
                e.add((min(a, b), max(a, b)))
    return sorted(e)


def hypercube(k):
    n = 1 << k
    e = set()
    for i in range(n):
        for b in range(k):
            j = i ^ (1 << b)
            e.add((min(i, j), max(i, j)))
    return n, sorted(e)


class P:
    def __init__(s):
        s.lo, s.up, s.c, s.it, s.rows, s.rlo, s.rup = [], [], [], [], [], [], []

    def var(s, lo, up, c, it=True):
        s.lo.append(lo); s.up.append(up); s.c.append(c); s.it.append(it)
        return len(s.lo) - 1

    def row(s, r, l, u):
        s.rows.append(r); s.rlo.append(l); s.rup.append(u)


def gen(seed):
    g = random.Random(seed)
    kind = g.choice(["graph", "assign", "knap", "maxcut", "bisect", "intrefl", "circcut", "cutcover"])
    p = P()
    if kind == "graph":
        if g.random() < 0.6:
            n = g.randint(9, 30) * SCALE
            e = circulant(n, sorted(set(g.randint(1, n // 2) for _ in range(g.randint(1, 3)))))
        else:
            n, e = hypercube(g.randint(3, 5))
        sub = g.choice(["stable", "cover", "dom"])
        xs = [p.var(0, 1, -1.0 if sub == "stable" else 1.0) for _ in range(n)]
        if sub == "stable":
            for a, b in e:
                p.row([(xs[a], 1.0), (xs[b], 1.0)], -inf, 1.0)
        elif sub == "cover":
            for a, b in e:
                p.row([(xs[a], 1.0), (xs[b], 1.0)], 1.0, inf)
        else:
            adj = [[j] for j in range(n)]
            for a, b in e:
                adj[a].append(b); adj[b].append(a)
            for j in range(n):
                p.row([(xs[k], 1.0) for k in adj[j]], 1.0, inf)
    elif kind == "assign":
        ni = g.randint(3, 8) * SCALE; nm = g.randint(2, 4)
        w = [g.randint(1, 9) for _ in range(ni)]; pr = [g.randint(1, 20) for _ in range(ni)]
        cap = g.randint(5, 20)
        x = {(i, m): p.var(0, 1, -float(pr[i])) for i in range(ni) for m in range(nm)}
        load = g.random() < 0.5
        for i in range(ni):
            p.row([(x[i, m], 1.0) for m in range(nm)], -inf, 1.0)
        for m in range(nm):
            r = [(x[i, m], float(w[i])) for i in range(ni)]
            if load:
                y = p.var(0, cap, 0.1, False)
                p.row(r + [(y, -1.0)], 0.0, 0.0)
            else:
                p.row(r, -inf, float(cap))
    elif kind == "knap":
        nt = g.randint(2, 4); per = g.randint(2, 4) * SCALE
        xs = []
        rows = [[] for _ in range(g.randint(1, 3))]
        caps = [g.randint(10, 40) for _ in rows]
        for t in range(nt):
            pr = g.randint(1, 15); ub = g.randint(1, 4); ws = [g.randint(1, 9) for _ in rows]
            for _ in range(per):
                j = p.var(0, ub, -float(pr))
                for k in range(len(rows)):
                    rows[k].append((j, float(ws[k])))
        for r, cp in zip(rows, caps):
            p.row(r, -inf, float(cp))
    elif kind in ("maxcut", "bisect", "circcut", "cutcover"):
        n = g.randint(6, 14) * SCALE
        if kind == "circcut":
            e = circulant(n, sorted(set(g.randint(1, n // 2) for _ in range(g.randint(1, 2)))))
            wt = {ed: 1.0 for ed in e}
        else:
            e = sorted({tuple(sorted(g.sample(range(n), 2))) for _ in range(g.randint(n, 3 * n))})
            wt = {ed: float(g.randint(-3, 9)) for ed in e}
        xs = [p.var(0, 1, 0.0) for _ in range(n)]
        for (a, b) in e:
            y = p.var(0, 1, -wt[(a, b)])
            # y = x_a xor x_b の 4 本 (反転で写り合う)
            p.row([(y, 1.0), (xs[a], -1.0), (xs[b], -1.0)], -inf, 0.0)
            p.row([(y, 1.0), (xs[a], 1.0), (xs[b], 1.0)], -inf, 2.0)
            if kind != "maxcut" or g.random() < 0.5:
                p.row([(y, 1.0), (xs[a], -1.0), (xs[b], 1.0)], 0.0, inf)
                p.row([(y, 1.0), (xs[a], 1.0), (xs[b], -1.0)], 0.0, inf)
        if kind == "bisect":
            p.row([(j, 1.0) for j in xs], float(n // 2), float(n - n // 2))
        if kind == "cutcover":
            # 各頂点で切れる辺が 1 本以上 (反転で不変)
            for v in range(n):
                inc = [len(xs) + k for k, ed in enumerate(e) if v in ed]
                if inc:
                    p.row([(j, 1.0) for j in inc], 1.0, inf)
    else:  # intrefl
        n = g.randint(4, 9) * SCALE; U = g.randint(2, 5)
        xs = [p.var(0, U, 0.0) for _ in range(n)]
        e = sorted({tuple(sorted(g.sample(range(n), 2))) for _ in range(g.randint(n, 2 * n))})
        for (a, b) in e:
            d = p.var(0, U, float(g.randint(-2, 3)), g.random() < 0.5)
            # d >= |x_a - x_b| (d は反転で動かない)
            p.row([(d, 1.0), (xs[a], -1.0), (xs[b], 1.0)], 0.0, inf)
            p.row([(d, 1.0), (xs[a], 1.0), (xs[b], -1.0)], 0.0, inf)
        # 中心のまわりで対称な和の制約
        k = g.randint(0, n * U // 4)
        p.row([(j, 1.0) for j in xs], float(k), float(n * U - k))
    return p, kind


def highs(p):
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.setOptionValue('mip_rel_gap', 1e-9)
    K = highspy.kHighsInf
    for j in range(len(p.lo)):
        h.addVar(p.lo[j] if p.lo[j] > -inf else -K, p.up[j] if p.up[j] < inf else K); h.changeColCost(j, p.c[j])
        if p.it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in zip(p.rows, p.rlo, p.rup):
        h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
    h.run(); return h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value


def ours(p):
    M = Model(); xs = [Variable(int if p.it[j] else float, p.lo[j], p.up[j]) for j in range(len(p.lo))]
    M.set_objective(sum(cc * x for cc, x in zip(p.c, xs)))
    for r, l, u in zip(p.rows, p.rlo, p.rup):
        e = sum(v * xs[j] for j, v in r)
        if l == u: M.add_constraint(e == l)
        else:
            if l > -inf: M.add_constraint(e >= l)
            if u < inf: M.add_constraint(e <= u)
    s = M.solve(mip_rel_gap=1e-9)
    return s.status, s.objective


if __name__ == '__main__':
    N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    bad = 0; t0 = time.time(); kinds = {}
    for seed in range(start, start + N):
        p, kind = gen(seed)
        kinds[kind] = kinds.get(kind, 0) + 1
        hs, ho = highs(p); os_, oo = ours(p)
        hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
        ok = (hmap == os_) and (hmap != 'optimal' or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
        if not ok:
            bad += 1
            print("BAD seed", seed, kind, "highs", hs, ho, "ours", os_, oo, flush=True)
    print("bad", bad, "of", N, "time %.1f" % (time.time() - t0), kinds)
