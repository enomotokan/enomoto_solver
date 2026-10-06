"""乱数で小さな MIP を作り、HiGHS と最適値を突き合わせる (python scripts/mip_fuzz_vs_highs.py 問題数 [開始seed])。
比較は ENOMOTO_MIP_REL_GAP=1e-9 で行うこと (既定の相対ギャップ 1e-4 では最適値が少しずれうる)。"""
import random, sys, highspy, time
from enomoto_solver import Model, Variable
inf=float('inf')
def gen(seed):
    g=random.Random(seed); n=g.randint(3,25); m=g.randint(2,15)
    lo=[];up=[];c=[];it=[]
    for j in range(n):
        t=g.randrange(5)
        if t==0: l,u=0.0,inf
        elif t==1: l=-float(g.randrange(5)); u=l+1+g.randrange(10)
        elif t in (2,3): l,u=0.0,1.0
        else: l,u=-5.0,inf
        lo.append(l);up.append(u);c.append(round((g.random()-0.5)*20,2)); it.append(g.random()<0.7)
    x0=[]
    for j in range(n):
        v=(lo[j]+(up[j]-lo[j])*g.random()) if up[j]<inf else lo[j]+3*g.random()
        if it[j]: v=round(v); v=min(max(v,lo[j]),up[j])
        x0.append(v)
    rows=[];rlo=[];rup=[]
    for i in range(m):
        r={}
        for _ in range(2+g.randrange(min(n,6))):
            j=g.randrange(n); v=float(round((g.random()-0.5)*20))
            if v!=0: r[j]=v
        r=list(r.items())
        if not r: continue
        act=sum(v*x0[j] for j,v in r)
        t=g.randrange(4)
        if t==0: rlo.append(-inf); rup.append(act+g.random()*3)
        elif t==1: rlo.append(act-g.random()*3); rup.append(inf)
        elif t==2: rlo.append(act-g.random()*2); rup.append(act+g.random()*2)
        else: rlo.append(act); rup.append(act)
        rows.append(r)
    # 有界にするため、全変数の和の上限を加える
    rows.append([(j,1.0) for j in range(n)]); rlo.append(-inf); rup.append(sum(abs(v) for v in x0)+20)
    rows.append([(j,1.0) for j in range(n)]); rlo.append(-sum(abs(v) for v in x0)-20); rup.append(inf)
    return lo,up,c,it,rows,rlo,rup
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
N=int(sys.argv[1]); start=int(sys.argv[2]) if len(sys.argv)>2 else 0
bad=0; t0=time.time()
for seed in range(start,start+N):
    lp=gen(seed)
    hs,ho=highs(lp); os_,oo,x=ours(lp)
    hmap={'Optimal':'optimal','Infeasible':'infeasible','Unbounded':'unbounded'}.get(hs,hs)
    ok = (hmap==os_) and (hmap!='optimal' or abs(ho-oo)<=1e-6*(1+abs(ho)))
    if not ok:
        bad+=1
        if bad<=10: print("seed",seed,"highs",hs,ho,"ours",os_,oo)
print("bad",bad,"of",N, "time %.1f"%(time.time()-t0))
