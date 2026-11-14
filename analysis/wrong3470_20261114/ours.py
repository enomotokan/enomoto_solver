"""ENOMOTO で seed 3470 を解く。FIX=1 で HiGHS の整数値に整数変数を固定 (上下限で)。"""
import os, sys
src=open(os.path.join(os.path.dirname(os.path.abspath(__file__)),'../../scripts/mip_fuzz_vs_highs.py')).read().split("N=int(sys.argv[1])")[0]
g={}; exec(src,g)
from enomoto_solver import Model, Variable
inf=float('inf')
lo,up,c,it,rows,rlo,rup=g['gen'](int(os.environ.get('SEED','3470')))
hx=[0.463618, -5.0, 0.0, 1.0, 5.0, 0.0, 1.0, 0.0, 2.0, 0.846907, 0.183733, 1.0, 0.045239, -0.0, 5.0, 37.20303, 1.0, 1.0, 1.0, -5.0, 1.0]
fix=[int(t) for t in os.environ.get('FIX','').split(',') if t]
M=Model(); xs=[]
for j in range(len(lo)):
    l,u=lo[j],up[j]
    if j in fix: l=u=round(hx[j])
    xs.append(Variable(int if it[j] else float,l,u))
M.set_objective(sum(cc*x for cc,x in zip(c,xs)))
for r,l,u in zip(rows,rlo,rup):
    e=sum(v*xs[j] for j,v in r)
    if l==u: M.add_constraint(e==l)
    else:
        if l>-inf: M.add_constraint(e>=l)
        if u<inf: M.add_constraint(e<=u)
s=M.solve(mip_rel_gap=1e-9)
print(s.status, s.objective)
