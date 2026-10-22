"""scripts/mip_fuzz_vs_highs.py の生成に幅の小さい連続列を混ぜた乱数の MIP を HiGHS と突き合わせる。
連続列の 4 割を [x0_j, x0_j + w] (w は 1e-9, 2e-9, 1e-8, 1e-7, 5e-7 から) にする (x0 は行を作る点なので実行可能性は保つ)。
使い方: python fuzz_mixtiny.py 問題数 [開始seed]  (比較は mip_rel_gap=1e-9)"""
import random, sys, time
from pathlib import Path
src = open(Path(__file__).resolve().parents[2] / "scripts/mip_fuzz_vs_highs.py").read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, "mip_fuzz_vs_highs.py", "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
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
    h=random.Random(seed*7919+1)
    for j in range(n):
        if not it[j] and h.random()<0.4:
            w=h.choice([1e-9,2e-9,1e-8,1e-7,5e-7]); base=h.choice([0.0, x0[j]])
            lo[j]=base; up[j]=base+w; x0[j]=base
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
    rows.append([(j,1.0) for j in range(n)]); rlo.append(-inf); rup.append(sum(abs(v) for v in x0)+20)
    rows.append([(j,1.0) for j in range(n)]); rlo.append(-sum(abs(v) for v in x0)-20); rup.append(inf)
    return lo,up,c,it,rows,rlo,rup
N=int(sys.argv[1]); start=int(sys.argv[2]) if len(sys.argv)>2 else 0
bad=0; t0=time.time()
for seed in range(start,start+N):
    lp=gen(seed); hs,ho=highs(lp); os_,oo,x=ours(lp)
    hmap={"Optimal":"optimal","Infeasible":"infeasible"}.get(hs,hs)
    ok=(hmap==os_) and (hmap!="optimal" or abs(ho-oo)<=1e-6*(1+abs(ho)))
    if not ok:
        bad+=1
        if bad<=20: print("seed",seed,"highs",hs,ho,"ours",os_,oo,flush=True)
print("bad",bad,"of",N,"time %.1f"%(time.time()-t0))
