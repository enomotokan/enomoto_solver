"""同じ整数列 (列ベクトル・費用・境界が一致) を含む乱数の MIP で HiGHS と最適値を突き合わせる
(ENOMOTO_PRESOLVE_PARALLEL_INT の確かめ用)。python fuzz_dupcols.py 問題数 [開始seed]"""
import random, sys, highspy
from enomoto_solver import Model, Variable
inf = float('inf')

def gen(seed):
    g = random.Random(seed)
    n0 = g.randint(3, 10); m = g.randint(2, 8)
    cols = []
    for _ in range(n0):
        u = g.choice([1, 1, 2, 3, 5])
        integer = g.random() < 0.8
        col = {i: g.randint(-5, 9) for i in range(m) if g.random() < 0.6}
        cost = g.randint(-10, 10)
        k = 1 + (g.randint(1, 3) if g.random() < 0.5 else 0)
        for _ in range(k):
            cols.append((integer, 0.0, float(u), cost, col))
    g.shuffle(cols)
    rows = []
    for i in range(m):
        r = [(j, float(c[4][i])) for j, c in enumerate(cols) if i in c[4] and c[4][i] != 0]
        kind = g.random()
        rhs = float(g.randint(0, 15))
        if kind < 0.6: rows.append((r, -inf, rhs))
        elif kind < 0.85: rows.append((r, rhs - g.randint(3, 10), inf))
        else: rows.append((r, rhs, rhs))
    return cols, rows

def highs(cols, rows):
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.setOptionValue('mip_rel_gap', 1e-9)
    K = highspy.kHighsInf
    for j, (it, lo, up, c, _) in enumerate(cols):
        h.addVar(lo, up); h.changeColCost(j, c)
        if it: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in rows:
        h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
    h.run()
    st = h.modelStatusToString(h.getModelStatus())
    return st, h.getInfo().objective_function_value

def ours(cols, rows):
    M = Model(); xs = [Variable(int if it else float, lo, up) for it, lo, up, _, _ in cols]
    M.set_objective(sum(c[3] * x for c, x in zip(cols, xs)))
    for r, l, u in rows:
        if not r: continue
        e = sum(v * xs[j] for j, v in r)
        if l == u: M.add_constraint(e == l)
        else:
            if l > -inf: M.add_constraint(e >= l)
            if u < inf: M.add_constraint(e <= u)
    s = M.solve(mip_rel_gap=1e-9, log=False)
    ok = True
    if s.status == 'optimal':
        x = [v.value for v in xs]
        for j, (it, lo, up, _, _) in enumerate(cols):
            if x[j] < lo - 1e-6 or x[j] > up + 1e-6 or (it and abs(x[j] - round(x[j])) > 1e-6): ok = False
        for r, l, u in rows:
            a = sum(v * x[j] for j, v in r)
            if a < l - 1e-6 or a > u + 1e-6: ok = False
    return s.status, s.objective, ok

N = int(sys.argv[1]); s0 = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0
for seed in range(s0, s0 + N):
    cols, rows = gen(seed)
    if any(not r for r, _, _ in rows):
        rows = [t for t in rows if t[0]]
    hs, ho = highs(cols, rows); os_, oo, feas = ours(cols, rows)
    hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
    ok = feas and hmap == os_ and (hmap != 'optimal' or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print('seed', seed, 'highs', hs, ho, 'ours', os_, oo, 'feasible', feas)
print('bad', bad, 'of', N)
