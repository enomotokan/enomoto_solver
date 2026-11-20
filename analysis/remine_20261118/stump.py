import os
HERE = os.path.dirname(os.path.abspath(__file__)) + "/"
DIR = os.environ.get("REMINE_TMP", "/tmp/remine_20261118/")  # 中間の pickle (リポジトリに入れない)
os.makedirs(DIR, exist_ok=True)
import json, math, pickle, re, sys, random
SP=DIR
K=json.load(open("analysis/binheur_20261111/kinds.json"))
F={}
for p,v in K.items():
    n,m=v["n"],v["m"]; rows=v.get("rows",{})
    f={"log_n":math.log10(n),"log_m":math.log10(m),"m_over_n":m/n,"bin_frac":v["bin"]/n,"genint":v["genint"],
       "cont_frac":v["cont"]/n}
    for t in ("partition","packing","cover","knapsack","other"):
        f[t+"_frac"]=rows.get(t,0)/m
    f["setpp_frac"]=f["partition_frac"]+f["packing_frac"]+f["cover_frac"]
    F[p]=f
for l in open("analysis/gate2_20261117/tab/feat.jsonl"):
    d=json.loads(l)
    if d.get("tag")!="def" or d["problem"] not in F: continue
    f=F[d["problem"]]
    o,b=d.get("objective"),d.get("root_bound")
    if o is not None and b is not None: f["root_gap"]=abs(o-b)/max(1e-9,abs(o),abs(b))
    if d.get("tab"):
        mm=re.search(r"mean agg nnz (\d+), LP rows (\d+) nnz (\d+), cols (\d+)",d["tab"][0])
        if mm:
            a,r,z,c=map(int,mm.groups()); f["log_lpnnz"]=math.log10(z+1); f["agg_nnz_per_col"]=a/max(c,1); f["lp_density"]=z/max(1,r*c)
    f["root_lpit_log"]=math.log10((d.get("lp_iterations") or 0)+1)
for _p in list(F):
    for _tl in (10, 20, 30, 45, 90, 120, 180, 300, 600, 900, 1200, 1800, 3600):
        F[f"{_p}@{_tl}"] = F[_p]  # 上限の違う回も同じ特徴
FEATS=sorted({k for f in F.values() for k in f})
out=pickle.load(open(SP+"scan.pkl","rb"))
def stump_fit(P,val):
    # val[p]: mean lr; choose feature, thr, dir minimizing sum of val over selected (selected = apply trial)
    best=(sum(min(0,0) for p in P),None,None,None) # apply none -> 0
    best=(0.0,None,None,None)
    tot=sum(val[p] for p in P)
    if tot<best[0]: best=(tot,"ALL",None,None)
    for ft in FEATS:
        Q=[p for p in P if ft in F.get(p,{})]
        if len(Q)<len(P)*0.8: continue
        Q.sort(key=lambda p:F[p][ft])
        pre=0.0
        for i in range(len(Q)-1):
            pre+=val[Q[i]]
            if F[Q[i]][ft]==F[Q[i+1]][ft]: continue
            thr=(F[Q[i]][ft]+F[Q[i+1]][ft])/2
            suf=sum(val[q] for q in Q[i+1:])
            if pre<best[0]: best=(pre,ft,thr,"<=")
            if suf<best[0]: best=(suf,ft,thr,">")
    return best
def sel(rule,p):
    _,ft,thr,dr=rule
    if ft is None: return False
    if ft=="ALL": return True
    if ft not in F.get(p,{}): return False
    return F[p][ft]<=thr if dr=="<=" else F[p][ft]>thr
res=[]
want=sys.argv[1:]
for r in out:
    if r["n"]<40: continue
    tag=r["dir"][9:]+":"+r["cfg"]
    if want and not any(w in tag for w in want): continue
    L=r["L"]; P=[p for p in L if p in F]
    if len(P)<35: continue
    S=sorted({s for p in P for s in L[p]})
    cv=[];rules=[]
    for s in S:
        Ptr=[p for p in P if any(t!=s for t in L[p])]
        val={p:sum(y for t,y in L[p].items() if t!=s)/len([t for t in L[p] if t!=s]) for p in Ptr}
        rule=stump_fit(Ptr,val); rules.append(rule)
        for p in P:
            if s in L[p]: cv.append(L[p][s] if sel(rule,p) else 0.0)
    full={p:sum(L[p].values())/len(L[p]) for p in P}
    rule=stump_fit(P,full)
    nsel=sum(sel(rule,p) for p in P)
    ins=math.exp(rule[0]/len(P))
    res.append((math.exp(sum(cv)/len(cv)),tag,r["tot"],r["cv"],rule,nsel,len(P),ins,[(x[1],None if x[2] is None else round(x[2],3),x[3]) for x in rules]))
res.sort()
for x in res:
    rule=x[4]
    print(f"{x[1]:40s} tot {x[2]:.3f} cvsel {x[3]:.3f} stumpCV {x[0]:.3f} | 全種で選ぶ: {rule[1]} {rule[3]} {rule[2] if rule[2] is None else round(rule[2],3)} ({x[5]}/{x[6]} 問, 見かけ {x[7]:.3f}) | 種ごと {x[8]}")
