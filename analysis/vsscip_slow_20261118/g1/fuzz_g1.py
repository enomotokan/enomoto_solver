"""群 1 の試作 (ENOMOTO_MIP_VUBAGG・ENOMOTO_MIP_ORBPP) の誤答の確かめ: 形を狙った乱数の小さな MIP を HiGHS と突き合わせる。
python fuzz_g1.py 種類 問題数 [開始seed]。種類 v = VUB の行と被覆の行 (VUBAGG が組を作る形)、c = 彩色・パッキング (パッキングのオービトープ)。
環境変数は呼ぶ側で付ける (例: ENOMOTO_MIP_VUBAGG=1)。解の実行可能性 (行・境界・整数性) も確かめる。"""
import random, sys, time, highspy
from enomoto_solver import Model, Variable
inf = float("inf")

def gen_v(seed):
    g = random.Random(seed)
    K = g.randint(1, 6); lo = []; up = []; c = []; it = []; rows = []; rlo = []; rup = []
    def var(l, u, cost, integer=True):
        lo.append(l); up.append(u); c.append(cost); it.append(integer); return len(lo) - 1
    xs = []
    for k in range(K):
        y = var(0, 1, float(g.randint(0, 5)))
        S = [var(0, 1, float(g.choice([0, 0, 0, -1, -2, 1, 2]))) for _ in range(g.randint(1, 6))]
        xs += S
        a = [float(g.choice([1, 1, 2, 3])) for _ in S]
        M = sum(a) if g.random() < 0.7 else max(1.0, sum(a) - g.randint(0, 3))
        if g.random() < 0.5:
            rows.append([(j, aj) for j, aj in zip(S, a)] + [(y, -M)]); rlo.append(-inf); rup.append(0.0)
        else:
            rows.append([(j, -aj) for j, aj in zip(S, a)] + [(y, M)]); rlo.append(0.0); rup.append(inf)
    # 被覆の行 (x の下向きの錠)、ときどき x の上向きの錠を足す行・一般整数・連続列
    for _ in range(g.randint(1, 5)):
        T = g.sample(xs, min(len(xs), g.randint(1, 5)))
        rows.append([(j, 1.0) for j in T]); rlo.append(float(g.randint(0, max(1, len(T) - 1)))); rup.append(inf)
    for _ in range(g.randint(0, 2)):
        T = g.sample(xs, min(len(xs), g.randint(1, 3)))
        z = var(0, g.randint(1, 4), float(g.randint(-2, 2)), g.random() < 0.6)
        rows.append([(j, 1.0) for j in T] + [(z, -1.0)]); rlo.append(-inf); rup.append(float(g.randint(0, 2)))
    return lo, up, c, it, rows, rlo, rup

def gen_c(seed):
    g = random.Random(seed)
    n = g.randint(3, 8); K = g.randint(3, min(n, 5)); E = [(u, v) for u in range(n) for v in range(u + 1, n) if g.random() < 0.4]
    lo = []; up = []; c = []; it = []; rows = []; rlo = []; rup = []
    def var(l, u, cost):
        lo.append(l); up.append(u); c.append(cost); it.append(True); return len(lo) - 1
    mode = g.randrange(3)
    wv = [float(g.randint(1, 5)) for _ in range(n)]
    x = [[var(0, 1, -wv[v] if mode == 1 else 0.0) for _ in range(K)] for v in range(n)]
    w = [var(0, 1, 1.0 if mode != 1 else 0.5) for _ in range(K)]
    for v in range(n):
        rows.append([(x[v][k], 1.0) for k in range(K)])
        if mode == 1: rlo.append(-inf); rup.append(1.0)
        else: rlo.append(1.0); rup.append(1.0)
    for (u, v) in E:
        for k in range(K):
            rows.append([(x[u][k], 1.0), (x[v][k], 1.0), (w[k], -1.0)]); rlo.append(-inf); rup.append(0.0)
    if mode == 2:  # 色の数の下限 (対称)
        rows.append([(w[k], 1.0) for k in range(K)]); rlo.append(float(g.randint(1, K))); rup.append(inf)
    return lo, up, c, it, rows, rlo, rup

def highs(lp):
    lo, up, c, it, rows, rlo, rup = lp
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("mip_rel_gap", 1e-9); K = highspy.kHighsInf
    for j in range(len(lo)):
        h.addVar(lo[j], up[j]); h.changeColCost(j, c[j])
        if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in zip(rows, rlo, rup): h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
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
    for j, v in enumerate(x):
        if v < lo[j] - 1e-6 or v > up[j] + 1e-6 or (it[j] and abs(v - round(v)) > 1e-6): return False
    return all(rl - 1e-6 <= sum(a * x[j] for j, a in r) <= ru + 1e-6 for r, rl, ru in zip(rows, rlo, rup))

kind, N = sys.argv[1], int(sys.argv[2]); start = int(sys.argv[3]) if len(sys.argv) > 3 else 0
gen = {"v": gen_v, "c": gen_c}[kind]
bad = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed); hs, ho = highs(lp); os_, oo, x = ours(lp)
    hm = {"Optimal": "optimal", "Infeasible": "infeasible"}.get(hs, hs)
    ok = hm == os_ and (hm != "optimal" or (abs(ho - oo) <= 1e-6 * (1 + abs(ho)) and feasible(lp, x)
                                            and abs(sum(cc * v for cc, v in zip(lp[2], x)) - oo) <= 1e-6 * (1 + abs(oo))))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, "feas", x is not None and feasible(lp, x))
print(kind, "bad", bad, "of", N, "time %.1f" % (time.time() - t0))
