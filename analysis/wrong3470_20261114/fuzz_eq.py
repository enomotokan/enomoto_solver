"""seed 3470 と同じ型の問題を乱数で作り HiGHS と最適値を突き合わせる (python3 fuzz_eq.py 問題数 [開始seed])。
scripts/mip_fuzz_vs_highs.py の生成に、(1) 等式行を多め、(2) 0-1 列どうしの組の等式 (x_a + x_b = 1 / x_a - x_b = 0、
基準の点 x0 で成り立つもの)、(3) 右辺が非整数の連続列を含む等式行、を足す。プロービングの同値の組・幅の小さい列の固定・
整数の二項等式の代入が重なる形。ENOMOTO_MIP_REL_GAP=1e-9 で回すこと。"""
import random, sys, os, time, io, contextlib
fz={}; exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)),'../../scripts/mip_fuzz_vs_highs.py')).read().split("N=int(sys.argv[1])")[0],fz)
highs, ours = fz['highs'], fz['ours']
inf=float('inf')
def gen(seed):
    g=random.Random(seed); n=g.randint(4,18); m=g.randint(2,9)
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
        for _ in range(2+g.randrange(min(n,5))):
            j=g.randrange(n); v=float(round((g.random()-0.5)*20))
            if v!=0: r[j]=v
        r=list(r.items())
        if not r: continue
        act=sum(v*x0[j] for j,v in r)
        t=g.randrange(6)
        if t==0: rlo.append(-inf); rup.append(act+g.random()*3)
        elif t==1: rlo.append(act-g.random()*3); rup.append(inf)
        elif t==2: rlo.append(act-g.random()*2); rup.append(act+g.random()*2)
        else: rlo.append(act); rup.append(act)
        rows.append(r)
    bins=[j for j in range(n) if it[j] and lo[j]==0 and up[j]==1]
    for _ in range(g.randrange(4) if len(bins)>=2 else 0):
        a,b=g.sample(bins,2)
        if x0[a]==x0[b]: rows.append([(a,1.0),(b,-1.0)]); rlo.append(0.0); rup.append(0.0)
        else: rows.append([(a,1.0),(b,1.0)]); rlo.append(1.0); rup.append(1.0)
    rows.append([(j,1.0) for j in range(n)]); rlo.append(-inf); rup.append(sum(abs(v) for v in x0)+20)
    rows.append([(j,1.0) for j in range(n)]); rlo.append(-sum(abs(v) for v in x0)-20); rup.append(inf)
    return lo,up,c,it,rows,rlo,rup
N=int(sys.argv[1]); start=int(sys.argv[2]) if len(sys.argv)>2 else 0
bad=0; t0=time.time(); cnt={}
for seed in range(start,start+N):
    lp=gen(seed)
    hs,ho=highs(lp)
    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
        os_,oo,x=ours(lp)
    hmap={'Optimal':'optimal','Infeasible':'infeasible','Unbounded':'unbounded'}.get(hs,hs)
    cnt[hmap]=cnt.get(hmap,0)+1
    ok = (hmap==os_) and (hmap!='optimal' or abs(ho-oo)<=1e-6*(1+abs(ho)))
    if not ok:
        bad+=1
        if bad<=20: print("seed",seed,"highs",hs,ho,"ours",os_,oo, flush=True)
print("bad",bad,"of",N,"highs status",cnt,"time %.1f"%(time.time()-t0))
