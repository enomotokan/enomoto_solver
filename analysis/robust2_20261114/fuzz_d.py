"""型 D の試作 (ENOMOTO_MIP_ORDINV) の誤答の確かめ: 前処理の効く形 (平行な列・二項等式・列シングルトン・集合詰め込み行) を
わざと入れた乱数の MIP を作り、HiGHS の最適値と、ENOMOTO の (行・列を並べ替えた) 解の目的値・実行可能性を突き合わせる。
python fuzz_d.py 問題数 [開始seed]   (環境変数はそのまま渡る。ENOMOTO_MIP_ORDINV=edpq などを付けて回す)"""
import random, sys, time
import highspy
from enomoto_solver import Model, Variable
inf = float("inf")
def gen(seed):
    g = random.Random(seed)
    n = g.randint(6, 30); m = g.randint(3, 14)
    lo, up, c, it = [], [], [], []
    for j in range(n):
        t = g.randrange(5)
        if t == 0: l, u = 0.0, inf
        elif t == 1: l = -float(g.randrange(4)); u = l + 1 + g.randrange(8)
        elif t in (2, 3): l, u = 0.0, 1.0
        else: l, u = 0.0, float(g.randrange(1, 12))
        lo.append(l); up.append(u); c.append(round((g.random() - 0.5) * 20, 1)); it.append(g.random() < 0.6)
    x0 = []
    for j in range(n):
        v = (lo[j] + (up[j] - lo[j]) * g.random()) if up[j] < inf else lo[j] + 3 * g.random()
        if it[j]: v = min(max(round(v), lo[j]), up[j])
        x0.append(v)
    rows, rlo, rup = [], [], []
    def add(r, kind):
        act = sum(v * x0[j] for j, v in r)
        if kind == 0: rlo.append(-inf); rup.append(act + g.random() * 3)
        elif kind == 1: rlo.append(act - g.random() * 3); rup.append(inf)
        elif kind == 2: rlo.append(act - g.random() * 2); rup.append(act + g.random() * 2)
        else: rlo.append(act); rup.append(act)
        rows.append(r)
    for i in range(m):
        r = {}
        for _ in range(2 + g.randrange(min(n, 6))):
            j = g.randrange(n); v = float(g.choice([1, -1, 2, -3, 0.5, 1.5, 7, -0.25]))
            r[j] = v
        add(list(r.items()), g.randrange(4))
    # 二項等式 (x_a = k x_b + d の形、整数の組か連続列を含む組)
    for _ in range(g.randrange(4)):
        a, b = g.sample(range(n), 2); k = g.choice([1.0, -1.0, 2.0, 0.5])
        add([(a, 1.0), (b, -k)], 3)
    # 集合詰め込み行 (0-1 列)
    bins = [j for j in range(n) if it[j] and lo[j] == 0 and up[j] == 1]
    for _ in range(g.randrange(3)):
        if len(bins) >= 3:
            s = g.sample(bins, g.randint(2, min(5, len(bins))))
            if sum(x0[j] for j in s) <= 1: rows.append([(j, 1.0) for j in s]); rlo.append(-inf); rup.append(1.0)
    # 平行な連続列: 既存の連続列 j の写し (係数を s 倍、費用も s 倍か少しずらす)
    nn = n
    for _ in range(g.randrange(4)):
        cont = [j for j in range(nn) if not it[j]]
        if not cont: break
        j = g.choice(cont); s = g.choice([1.0, 2.0, -1.0, 3.0, 0.1, 1 / 3])
        q = len(lo)
        lo.append(0.0); up.append(g.choice([inf, float(g.randrange(1, 6))])); c.append(c[j] * s if g.random() < 0.7 else c[j] * s + 1.0); it.append(False); x0.append(0.0)
        for r in rows:
            for (jj, v) in list(r):
                if jj == j: r.append((q, v * s))
    n = len(lo)
    rows.append([(j, 1.0) for j in range(n)]); rlo.append(-inf); rup.append(sum(abs(v) for v in x0) + 20)
    rows.append([(j, 1.0) for j in range(n)]); rlo.append(-sum(abs(v) for v in x0) - 20); rup.append(inf)
    return lo, up, c, it, rows, rlo, rup
def highs(lp):
    lo, up, c, it, rows, rlo, rup = lp
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("mip_rel_gap", 1e-9)
    K = highspy.kHighsInf
    for j in range(len(lo)):
        h.addVar(lo[j] if lo[j] > -inf else -K, up[j] if up[j] < inf else K); h.changeColCost(j, c[j])
        if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in zip(rows, rlo, rup): h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
    h.run(); return h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value
def ours(lp, perm):
    lo, up, c, it, rows, rlo, rup = lp
    n = len(lo); g = random.Random(perm); cp = list(range(n)); rp = list(range(len(rows)))
    if perm: g.shuffle(cp); g.shuffle(rp)
    M = Model(); xs = [None] * n
    for j in cp: xs[j] = Variable(int if it[j] else float, lo[j], up[j])
    M.set_objective(sum(cc * x for cc, x in zip(c, xs)))
    for i in rp:
        r, l, u = rows[i], rlo[i], rup[i]
        rr = list(r); g.shuffle(rr)
        e = sum(v * xs[j] for j, v in rr)
        if l == u: M.add_constraint(e == l)
        else:
            if l > -inf: M.add_constraint(e >= l)
            if u < inf: M.add_constraint(e <= u)
    s = M.solve(mip_rel_gap=1e-9)
    x = [v.value for v in xs] if s.status == "optimal" else None
    if x is not None:  # 元の問題で実行可能か
        viol = max([0.0] + [max(lo[j] - x[j], x[j] - up[j]) for j in range(n)] + [abs(x[j] - round(x[j])) for j in range(n) if it[j]]
                   + [max(l - sum(v * x[j] for j, v in r), sum(v * x[j] for j, v in r) - u) for r, l, u in zip(rows, rlo, rup)])
        if viol > 1e-6: return "infeasible-sol", s.objective, viol
    return s.status, s.objective, 0.0
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed)
    hs, ho = highs(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible", "Unbounded": "unbounded"}.get(hs, hs)
    for perm in (0, 1, 2):
        os_, oo, viol = ours(lp, perm)
        ok = (hmap == os_ or (hmap in ("infeasible", "unbounded") and os_ in ("infeasible", "unbounded", "infeasible_or_unbounded"))) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
        if not ok:
            bad += 1
            if bad <= 10: print("seed", seed, "perm", perm, "highs", hs, ho, "ours", os_, oo, viol, flush=True)
print("bad", bad, "of", 3 * N, "time %.1f" % (time.time() - t0))
