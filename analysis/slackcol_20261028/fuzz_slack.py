"""費用 0・有界の連続列が 1 行だけに現れる (不等式・範囲制約・等式の行、1 行に何本も) 乱数の LP/MIP で HiGHS と最適値を
突き合わせる (ENOMOTO_PRESOLVE_SLACK_COL の確かめ用)。python fuzz_slack.py 問題数 [開始seed]"""
import random, sys, highspy
from enomoto_solver import Model, Variable
inf = float('inf')

def gen(seed):
    g = random.Random(seed)
    n = g.randint(2, 8); m = g.randint(1, 6)
    mip = g.random() < 0.7
    cols = [((mip and g.random() < 0.7), 0.0, float(g.choice([1, 2, 5, 10])), g.randint(-10, 10)) for _ in range(n)]
    rows = []
    for i in range(m):
        r = [(j, float(g.randint(-6, 9))) for j in range(n) if g.random() < 0.6]
        r = [t for t in r if t[1] != 0]
        mid = float(g.randint(-5, 15))
        kind = g.random()
        if kind < 0.3: lo, up = -inf, mid
        elif kind < 0.5: lo, up = mid, inf
        elif kind < 0.8: lo, up = mid - g.randint(0, 8), mid
        else: lo, up = mid, mid
        rows.append([r, lo, up])
    # スラック列: 費用 0、有界、1 行だけ (1 行に 1〜4 本、同じ係数のことも)
    for _ in range(g.randint(1, 6)):
        i = g.randrange(m); k = g.randint(1, 4); a = float(g.choice([-3, -1, 1, 2, 5]))
        for _ in range(k):
            j = len(cols)
            l = float(g.choice([0, 0, -1, 2])); u = l + float(g.choice([1, 3, 0.5]))
            cols.append((False, l, u, 0))
            rows[i][0].append((j, a if g.random() < 0.6 else float(g.randint(-4, 4) or 1)))
    return cols, [tuple(r) for r in rows if r[0]]

def highs(cols, rows):
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.setOptionValue('mip_rel_gap', 1e-9)
    K = highspy.kHighsInf
    for j, (it, lo, up, c) in enumerate(cols):
        h.addVar(lo, up); h.changeColCost(j, c)
        if it: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in rows:
        h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
    h.run()
    return h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value

def ours(cols, rows):
    M = Model(); xs = [Variable(int if it else float, lo, up) for it, lo, up, _ in cols]
    M.set_objective(sum(c[3] * x for c, x in zip(cols, xs)))
    for r, l, u in rows:
        e = sum(v * xs[j] for j, v in r)
        if l == u: M.add_constraint(e == l)
        else:
            if l > -inf: M.add_constraint(e >= l)
            if u < inf: M.add_constraint(e <= u)
    s = M.solve(mip_rel_gap=1e-9, log=False)
    ok = True
    if s.status == 'optimal':
        x = [v.value for v in xs]
        for j, (it, lo, up, _) in enumerate(cols):
            if x[j] < lo - 1e-6 or x[j] > up + 1e-6 or (it and abs(x[j] - round(x[j])) > 1e-6): ok = False
        for r, l, u in rows:
            a = sum(v * x[j] for j, v in r)
            if a < l - 1e-6 or a > u + 1e-6: ok = False
    return s.status, s.objective, ok

N = int(sys.argv[1]); s0 = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0
for seed in range(s0, s0 + N):
    cols, rows = gen(seed)
    hs, ho = highs(cols, rows); os_, oo, feas = ours(cols, rows)
    hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
    ok = feas and hmap == os_ and (hmap != 'optimal' or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print('seed', seed, 'highs', hs, ho, 'ours', os_, oo, 'feasible', feas)
print('bad', bad, 'of', N)
