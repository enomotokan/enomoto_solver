"""mc11 風の固定費用ネットワーク (単一品種、辺ごとに連続フロー x <= cap*y、y は 0-1) を乱数で作り、HiGHS と比べる。
usage: fcfuzz.py start count nn threads tl"""
import random, sys, time
import highspy
from enomoto_solver import benchmark_highs as bh
inf = highspy.kHighsInf

def gen(seed, nn):
    g = random.Random(seed)
    arcs = set()
    for v in range(1, nn):
        arcs.add((g.randrange(v), v))
    while len(arcs) < 4 * nn:
        a, b = g.randrange(nn), g.randrange(nn)
        if a != b: arcs.add((a, b))
    arcs = sorted(arcs)
    dem = [g.randint(-6, 8) for _ in range(nn)]
    nsrc = max(1, nn // 6)
    for s in range(nsrc): dem[s] = 0
    tot = sum(dem[nsrc:])
    for s in range(nsrc): dem[s] = -(tot // nsrc) - (1 if s < tot % nsrc else 0)
    h = highspy.Highs(); h.setOptionValue("output_flag", False)
    xi = {}
    for e in arcs:
        xi[e] = h.getNumCol(); h.addVar(0.0, float(g.randint(8, 30))); h.changeColCost(xi[e], float(g.randint(1, 6)))
    for e in arcs:
        j = xi[e]; cap = h.getLp().col_upper_[j]
        y = h.getNumCol(); h.addVar(0.0, 1.0); h.changeColCost(y, float(g.randint(20, 120))); h.changeColIntegrality(y, highspy.HighsVarType.kInteger)
        h.addRow(-inf, 0.0, 2, [j, y], [1.0, -cap])
    for v in range(nn):
        idx, val = [], []
        for e, j in xi.items():
            if e[1] == v: idx.append(j); val.append(1.0)
            if e[0] == v: idx.append(j); val.append(-1.0)
        if idx: h.addRow(float(dem[v]), float(dem[v]), len(idx), idx, val)
    return h

start, cnt, nn, th, tl = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]), float(sys.argv[5])
bad = 0
import os
REP = int(os.environ.get('REP', '1'))
for seed in range(start, start + cnt):
    h = gen(seed, nn)
    lp = h.getLp()
    t = time.time(); h.setOptionValue("mip_rel_gap", 0.0); h.setOptionValue("threads", 1); h.run(); th_ = time.time() - t
    hs = h.modelStatusToString(h.getModelStatus()); ho = h.getInfo().objective_function_value
    for k in range(REP):
        m, _, _ = bh._build_our_model(lp)
        t = time.time(); r = m.solve(None, False, tl, None, None, th); to = time.time() - t
        ok = not (r["status"] == "optimal" and hs == "Optimal" and abs(r["objective"] - ho) > 1e-6 * (1 + abs(ho)))
        ok = ok and not (r["status"] == "optimal" and hs != "Optimal")
        if not ok: bad += 1
        print(f"seed {seed}.{k} highs {hs} {ho} {th_:.1f}s ours {r['status']} {r['objective']} {to:.1f}s nodes {r['nodes']} {'BAD' if not ok else ''}", flush=True)
print("bad", bad)
