"""試作 ENOMOTO_MIP_INTSING を狙った fuzz: 等式 1 本にだけ現れる整数列 y_k (係数 ±1) を持つ小さな MIP を作り、HiGHS と最適値・状態を突き合わせる。
y_k = b_k ± Σ a x (x は 0-1 か小さな整数、a は小さな整数)、y_k の境界・費用はいろいろ。ほかに x の乱数の行。
使い方: ENOMOTO_MIP_INTSING=1 ENOMOTO_MIP_REL_GAP=1e-9 python fuzz_intsing.py 問題数 [開始 seed]"""
import random, sys, time
from pathlib import Path
import highspy
from enomoto_solver import Model, Variable
# 以下の highs・ours は scripts/mip_fuzz_vs_highs.py と同じ
def highs(lp):
    lo,up,c,it,rows,rlo,rup=lp
    h=highspy.Highs(); h.setOptionValue('output_flag',False); h.setOptionValue('mip_rel_gap',1e-9)
    K=highspy.kHighsInf
    for j in range(len(lo)):
        h.addVar(lo[j] if lo[j]>-inf else -K, up[j] if up[j]<inf else K); h.changeColCost(j,c[j])
        if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r,l,u in zip(rows,rlo,rup): h.addRow(l if l>-inf else -K,u if u<inf else K,len(r),[a for a,_ in r],[b for _,b in r])
    h.run(); return h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value
def ours(lp):
    lo,up,c,it,rows,rlo,rup=lp
    M=Model(); xs=[Variable(int if it[j] else float,lo[j],up[j]) for j in range(len(lo))]
    M.set_objective(sum(cc*x for cc,x in zip(c,xs)))
    for r,l,u in zip(rows,rlo,rup):
        e=sum(v*xs[j] for j,v in r)
        if l==u: M.add_constraint(e==l)
        else:
            if l>-inf: M.add_constraint(e>=l)
            if u<inf: M.add_constraint(e<=u)
    s=M.solve(mip_rel_gap=1e-9)
    return s.status, s.objective, [x.value for x in xs] if s.status=="optimal" else None
inf = float("inf")
def gen(seed):
    g = random.Random(seed); nx = g.randint(3, 14); ny = g.randint(1, 5)
    lo, up, c, it = [], [], [], []
    for j in range(nx):
        if g.random() < 0.7: l, u = 0.0, 1.0
        else: l, u = float(-g.randrange(3)), float(g.randrange(1, 5))
        lo.append(l); up.append(u); c.append(round((g.random() - 0.5) * 10, 1)); it.append(g.random() < 0.85)
    x0 = [round(lo[j] + (up[j] - lo[j]) * g.random()) if it[j] else lo[j] + (up[j] - lo[j]) * g.random() for j in range(nx)]
    rows, rlo, rup = [], [], []
    for k in range(ny):
        S = g.sample(range(nx), g.randint(1, min(nx, 6)))
        terms = [(j, float(g.choice([1, 1, 1, -1, 2, -2, 3]))) for j in S]
        sy = g.choice([1.0, -1.0]); b = float(g.randrange(-2, 3))
        val = (b - sum(a * x0[j] for j, a in terms)) / sy
        t = g.randrange(4)
        if t == 0: l, u = 0.0, 1.0
        elif t == 1: l, u = 0.0, inf
        elif t == 2: l, u = -inf, inf
        else: l, u = float(g.randrange(-3, 1)), float(g.randrange(1, 4))
        y = len(lo); lo.append(l); up.append(u); c.append(round((g.random() - 0.5) * 20, 1)); it.append(g.random() < 0.9)
        rows.append(terms + [(y, sy)]); rlo.append(b); rup.append(b)
    for i in range(g.randint(1, 6)):
        r = {}
        for _ in range(2 + g.randrange(min(nx, 5))):
            j = g.randrange(nx); v = float(round((g.random() - 0.5) * 10))
            if v: r[j] = v
        r = list(r.items())
        if not r: continue
        act = sum(v * x0[j] for j, v in r); t = g.randrange(3)
        if t == 0: rlo.append(-inf); rup.append(act + g.random() * 3)
        elif t == 1: rlo.append(act - g.random() * 3); rup.append(inf)
        else: rlo.append(act - 1); rup.append(act + 1)
        rows.append(r)
    n = len(lo)
    rows.append([(j, 1.0) for j in range(n)]); rlo.append(-60.0); rup.append(60.0)
    return lo, up, c, it, rows, rlo, rup
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed); hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible", "Unbounded": "unbounded"}.get(hs, hs)
    ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if hmap == "Primal infeasible or unbounded" and os_ in ("infeasible", "unbounded", "infeasible_or_unbounded"): ok = True
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0))
