"""対称性 (入れ替え・反転) のある乱数の LP / MIP で、畳んだ求解と HiGHS の最適値を突き合わせる (analysis/drcr_20261110.md)。

python analysis/drcr_20261110/fuzz_fold.py 問題数 [開始seed] [lp|mip]
畳みの切り替えは環境変数 (ENOMOTO_LP_FOLD・ENOMOTO_MIP_FOLD) をこのプロセスに付けて選ぶ。

作り方: 小さい元の問題 (列 nv、行 nr、整数と連続を混ぜる) を k 個写し、写しの間を結ぶ行 (同じ列の写しの和) を足す。
写しごとに確率で反転 (x = u − x' と置き換え、係数・費用の符号を変える) し、列・行の順を混ぜる。一部は輸送問題
(供給・需要が同じ値の組、費用が同じ組) にする (整数の組の畳み、段階 3 用)。一部では 1 つの係数を変えて対称性を崩す。
"""
import random, sys, time, highspy
from enomoto_solver import Model, Variable

inf = float('inf')
MODE = sys.argv[3] if len(sys.argv) > 3 else "mip"


def gen_copies(g):
    k = g.randint(2, 4); nv = g.randint(1, 4); nr = g.randint(1, 3)
    lo0 = [0.0] * nv; up0 = [float(g.randint(1, 4)) for _ in range(nv)]
    c0 = [float(g.randint(-5, 5)) for _ in range(nv)]
    it0 = [g.random() < 0.5 for _ in range(nv)]
    rows0 = []
    for _ in range(nr):
        r = [(j, float(g.choice([-3, -2, -1, 1, 1, 2, 3]))) for j in range(nv) if g.random() < 0.7]
        if not r: r = [(0, 1.0)]
        act = sum(a * (up0[j] / 2) for j, a in r)
        rows0.append((r, g.choice(['le', 'ge', 'eq']), float(round(act + g.randint(-2, 2)))))
    lo, up, c, it, rows, rlo, rup = [], [], [], [], [], [], []
    vid = {}
    flip = [g.random() < 0.3 for _ in range(k)]
    const = 0.0
    for cc in range(k):
        for j in range(nv):
            vid[cc, j] = len(lo)
            lo.append(lo0[j]); up.append(up0[j]); it.append(it0[j])
            c.append(-c0[j] if flip[cc] else c0[j])
            if flip[cc]: const += c0[j] * up0[j]
    def add(terms, sense, rhs):
        # terms: (列, 係数) を元の変数で。反転した写しの列は x = u − x'
        r = []; b = rhs
        for (cc, j), a in terms:
            if flip[cc]: r.append((vid[cc, j], -a)); b -= a * up0[j]
            else: r.append((vid[cc, j], a))
        l, u = {'le': (-inf, b), 'ge': (b, inf), 'eq': (b, b)}[sense]
        rows.append(r); rlo.append(l); rup.append(u)
    for cc in range(k):
        for r, s, b in rows0:
            add([((cc, j), a) for j, a in r], s, b)
    for j in range(nv):
        if g.random() < 0.6:
            tot = sum(up0[j] for _ in range(k))
            add([((cc, j), 1.0) for cc in range(k)], g.choice(['le', 'ge', 'eq']), float(round(tot * g.uniform(0.2, 0.8))))
    return lo, up, c, it, rows, rlo, rup


def gen_transport(g):
    ns = g.randint(2, 4); nd = g.randint(2, 4)
    s = g.randint(2, 6); d = g.randint(1, 5)
    cost = float(g.randint(1, 5))
    lo, up, c, it, rows, rlo, rup = [], [], [], [], [], [], []
    xid = {}
    for i in range(ns):
        for j in range(nd):
            xid[i, j] = len(lo); lo.append(0.0); up.append(float(g.randint(2, 3)) if g.random() < 0.0 else inf); c.append(cost if g.random() < 0.85 else cost + 1); it.append(True)
    for i in range(ns):
        rows.append([(xid[i, j], 1.0) for j in range(nd)]); rlo.append(-inf); rup.append(float(s))
    for j in range(nd):
        rows.append([(xid[i, j], 1.0) for i in range(ns)]); rlo.append(float(d)); rup.append(float(d) if g.random() < 0.5 else inf)
    return lo, up, c, it, rows, rlo, rup


def gen(seed):
    g = random.Random(seed)
    lp = gen_transport(g) if g.random() < 0.3 else gen_copies(g)
    lo, up, c, it, rows, rlo, rup = lp
    if g.random() < 0.15 and rows:
        r = g.randrange(len(rows)); i = g.randrange(len(rows[r])); rows[r][i] = (rows[r][i][0], rows[r][i][1] + 1.0)
    # 順を混ぜる
    n = len(lo); perm = list(range(n)); g.shuffle(perm); inv = {p: i for i, p in enumerate(perm)}
    lo = [lo[p] for p in perm]; up = [up[p] for p in perm]; c = [c[p] for p in perm]; it = [it[p] for p in perm]
    rows = [[(inv[j], a) for j, a in r] for r in rows]
    rp = list(range(len(rows))); g.shuffle(rp)
    rows = [rows[i] for i in rp]; rlo = [rlo[i] for i in rp]; rup = [rup[i] for i in rp]
    if MODE == "lp": it = [False] * n
    return lo, up, c, it, rows, rlo, rup


def highs(lp):
    lo, up, c, it, rows, rlo, rup = lp
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.setOptionValue('mip_rel_gap', 1e-9)
    K = highspy.kHighsInf
    for j in range(len(lo)):
        h.addVar(lo[j] if lo[j] > -inf else -K, up[j] if up[j] < inf else K); h.changeColCost(j, c[j])
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
    xv = [x.value for x in xs] if s.status == "optimal" else None
    return s.status, s.objective, xv


def feasible(lp, x):
    lo, up, c, it, rows, rlo, rup = lp
    for j in range(len(lo)):
        if x[j] < lo[j] - 1e-6 or x[j] > up[j] + 1e-6: return False
        if it[j] and abs(x[j] - round(x[j])) > 1e-6: return False
    for r, l, u in zip(rows, rlo, rup):
        a = sum(v * x[j] for j, v in r)
        if a < l - 1e-6 * (1 + abs(l)) or a > u + 1e-6 * (1 + abs(u)): return False
    return True


N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed)
    hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
    same_inf = hmap in ('infeasible', 'unbounded') and os_ in ('infeasible', 'unbounded', 'infeasible_or_unbounded')
    ok = same_inf or ((hmap == os_) and (hmap != 'optimal' or (abs(ho - oo) <= 1e-6 * (1 + abs(ho)) and feasible(lp, x))))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0))
