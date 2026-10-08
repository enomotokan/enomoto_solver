"""対称性 (同じ機械の入れ替え) をもつ乱数の割り当て問題で、HiGHS と最適値を突き合わせる
(python scripts/mip_fuzz_symmetric.py 問題数 [開始seed])。対称性の検出・オービトープの固定・lex-leader の行の
正しさを確かめる用 (mip_fuzz_vs_highs.py の乱数問題はほとんど対称性をもたない)。

仕事 i (重さ w_i、利益 p_i) を同じ機械 m (容量 cap) に割り当てる。各仕事は高々 1 台 (確率 1/2 でちょうど 1 台)。
一部の問題では機械ごとの負荷の連続変数 y_m = sum_i w_i x_{i,m} を加え (連続列も一緒に入れ替わる対称性)、
一部では仕事の衝突 (同じ機械に置けない組) を全ての機械に加え、一部では 1 台だけ容量を変えて対称性を崩す。"""
import random, sys, time, highspy
from enomoto_solver import Model, Variable

inf = float('inf')


def gen(seed):
    g = random.Random(seed)
    ni = g.randint(2, 8)
    nm = g.randint(2, 4)
    w = [g.randint(1, 9) for _ in range(ni)]
    p = [g.randint(1, 20) for _ in range(ni)]
    cap = [g.randint(5, 20)] * nm
    if g.random() < 0.2:
        cap[g.randrange(nm)] += g.randint(1, 5)
    use_load = g.random() < 0.5
    exact = g.random() < 0.5
    lo, up, c, it = [], [], [], []
    xid = {}
    for i in range(ni):
        for m in range(nm):
            xid[i, m] = len(lo)
            lo.append(0.0); up.append(1.0); c.append(-float(p[i])); it.append(True)
    yid = {}
    if use_load:
        for m in range(nm):
            yid[m] = len(lo)
            lo.append(0.0); up.append(inf); c.append(0.1); it.append(False)
    rows, rlo, rup = [], [], []
    for i in range(ni):
        rows.append([(xid[i, m], 1.0) for m in range(nm)])
        rlo.append(1.0 if exact else -inf); rup.append(1.0)
    for m in range(nm):
        r = [(xid[i, m], float(w[i])) for i in range(ni)]
        if use_load:
            rows.append(r + [(yid[m], -1.0)]); rlo.append(0.0); rup.append(0.0)
            rows.append([(yid[m], 1.0)]); rlo.append(-inf); rup.append(float(cap[m]))
        else:
            rows.append(r); rlo.append(-inf); rup.append(float(cap[m]))
    if g.random() < 0.5:
        for _ in range(g.randint(1, 3)):
            a, b = g.sample(range(ni), 2) if ni >= 2 else (0, 0)
            if a != b:
                for m in range(nm):
                    rows.append([(xid[a, m], 1.0), (xid[b, m], 1.0)]); rlo.append(-inf); rup.append(1.0)
    return lo, up, c, it, rows, rlo, rup


# HiGHS と自分のソルバーで解く (mip_fuzz_vs_highs.py と同じ)
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

N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time()
for seed in range(start, start + N):
    lp = gen(seed)
    hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded'}.get(hs, hs)
    ok = (hmap == os_) and (hmap != 'optimal' or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo)
print("bad", bad, "of", N, "time %.1f" % (time.time() - t0))
