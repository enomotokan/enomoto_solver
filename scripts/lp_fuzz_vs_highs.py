"""乱数で小さな LP (実行不能・非有界を含む) を作り、HiGHS と状態・最適値を突き合わせる
(python scripts/lp_fuzz_vs_highs.py 問題数 [simplex|auto|ipm_crossover])。"""
import random, sys, highspy, os
from enomoto_solver import Model, Variable
inf=float('inf')
def gen(seed):
    g=random.Random(seed); n=g.randint(4,20); m=g.randint(2,14)
    lo=[];up=[];c=[]
    for j in range(n):
        t=g.randrange(4)
        if t==0: l,u=0.0,inf
        elif t==1: l=-float(g.randrange(5)); u=l+1+g.randrange(10)
        elif t==2: l,u=0.0,1.0
        else: l,u=-5.0,inf
        lo.append(l);up.append(u);c.append((g.random()-0.3)*10)
    x0=[(lo[j]+(up[j]-lo[j])*g.random()) if up[j]<inf else lo[j]+3*g.random() for j in range(n)]
    rows=[];rlo=[];rup=[]
    feas = g.random()<0.8
    for i in range(m):
        r={}
        for _ in range(2+g.randrange(min(n,6))):
            j=g.randrange(n); v=float(round((g.random()-0.5)*20))
            if v!=0: r[j]=v
        r=list(r.items())
        act=sum(v*x0[j] for j,v in r)
        if not feas and g.random()<0.3: act+= (g.random()-0.5)*40
        t=g.randrange(3)
        if t==0: rlo.append(-inf); rup.append(act+g.random()*3)
        elif t==1: rlo.append(act-g.random()*3); rup.append(inf)
        else: rlo.append(act); rup.append(act)
        rows.append(r)
    return lo,up,c,rows,rlo,rup
def highs(lp):
    lo,up,c,rows,rlo,rup=lp
    h=highspy.Highs(); h.setOptionValue('output_flag',False)
    for j in range(len(lo)): h.addVar(lo[j] if lo[j]>-inf else -highspy.kHighsInf, up[j] if up[j]<inf else highspy.kHighsInf); h.changeColCost(j,c[j])
    for r,l,u in zip(rows,rlo,rup): h.addRow(l if l>-inf else -highspy.kHighsInf,u if u<inf else highspy.kHighsInf,len(r),[a for a,_ in r],[b for _,b in r])
    h.run(); s=h.modelStatusToString(h.getModelStatus()); return s, h.getInfo().objective_function_value
def ours(lp, solver):
    lo,up,c,rows,rlo,rup=lp
    M=Model(); xs=[Variable(float,l,u) for l,u in zip(lo,up)]
    M.set_objective(sum(cc*x for cc,x in zip(c,xs)))
    for r,l,u in zip(rows,rlo,rup):
        if not r: continue
        e=sum(v*xs[j] for j,v in r)
        if l==u: M.add_constraint(e==l)
        else:
            if l>-inf: M.add_constraint(e>=l)
            if u<inf: M.add_constraint(e<=u)
    s=M.solve(root_solver=solver, distinguish_infeasible_unbounded=True)
    return s.status, s.objective
N=int(sys.argv[1]); solver=sys.argv[2] if len(sys.argv)>2 else "simplex"
bad=0; cats={}
for seed in range(N):
    lp=gen(seed)
    if any(len(r)==0 for r in lp[3]): continue
    hs,ho=highs(lp); os_,oo=ours(lp,solver)
    hmap={'Optimal':'optimal','Infeasible':'infeasible','Unbounded':'unbounded'}.get(hs,hs)
    ok = (hmap==os_) and (hmap!='optimal' or abs(ho-oo)<=1e-6*(1+abs(ho)))
    if hs=='Primal infeasible or unbounded' and os_ in('infeasible','unbounded','infeasible_or_unbounded'): ok=True
    if not ok:
        bad+=1; k=(hmap,os_); cats[k]=cats.get(k,0)+1
        if bad<=8: print("seed",seed,"highs",hs,ho,"ours",os_,oo)
print("bad",bad,"of",N,cats)
