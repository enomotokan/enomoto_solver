import os
HERE = os.path.dirname(os.path.abspath(__file__)) + "/"
DIR = os.environ.get("REMINE_TMP", "/tmp/remine_20261118/")  # 中間の pickle (リポジトリに入れない)
os.makedirs(DIR, exist_ok=True)
import pickle, math, itertools, collections, sys
D = pickle.load(open(DIR+"D.pkl", "rb"))
NOISY = {"noswot","neos-911970","neos-787933","fiball","CMS750_4","rococoC10-001000","kasai","timtab1","mc11","pk1","10teams","rococoC10","neos-1171448","mzzv42z"}
BASES = ["def_t1","def", "base", "base2", "k", "c14", "cand1114", "c18", "cand1117", "ours", "c17_mr8", "full", "b"]
def isnoisy(p): return any(p.startswith(n) for n in NOISY)
def sgm_ratio(lr): return math.exp(sum(lr)/len(lr)) if lr else 1.0
def rank(v):
    o = sorted(range(len(v)), key=lambda i: v[i]); r=[0]*len(v)
    for k,i in enumerate(o): r[i]=k
    return r
def spear(a,b):
    n=len(a)
    if n<3: return float('nan')
    ra,rb=rank(a),rank(b)
    return 1-6*sum((x-y)**2 for x,y in zip(ra,rb))/(n*(n*n-1))
out=[]
for d0,R in D.items():
    cfgs=collections.Counter(k[1] for k in R)
    base=next((b for b in BASES if b in cfgs), None)
    if base is None: continue
    for c in cfgs:
        base=next((b for b in BASES if b in cfgs), None)
        if "robust_20261114" in d0 and c in ("pgq","spgq"): base="base2"
        if c in ("base2","old","def2","rev","def_v2","highs_cut","ours_cut") or c.endswith("_t1") and base=="def": continue
        if c==base or c in ("highs","scip") or c.startswith("cand"): continue
        # paired
        L=collections.defaultdict(dict)
        for (p,cc,s),v in R.items():
            if cc!=c: continue
            b=R.get((p,base,s))
            if b is None: continue
            L[p][s]=math.log((v[0]+1)/(b[0]+1))
        P=[p for p in L if len(L[p])>=2]
        if len(P)<8: continue
        allr=[x for p in P for x in L[p].values()]
        tot=sgm_ratio(allr)
        S=sorted({s for p in P for s in L[p]})
        fast=[p for p in P if all(x<math.log(0.8) for x in L[p].values()) and (not isnoisy(p) or len(L[p])>=3)]
        slow=[p for p in P if all(x>math.log(1.25) for x in L[p].values()) and (not isnoisy(p) or len(L[p])>=3)]
        # spearman across seed pairs on common problems
        rh=[]
        for a,b in itertools.combinations(S,2):
            Q=[p for p in P if a in L[p] and b in L[p]]
            if len(Q)>=5: rh.append(spear([L[p][a] for p in Q],[L[p][b] for p in Q]))
        rho=sum(rh)/len(rh) if rh else float('nan')
        # oracle in-sample: apply where mean<0
        orc=[ (x if sum(L[p].values())<0 else 0.0) for p in P for x in L[p].values()]
        # LOO-seed CV
        cv=[];cvs=[]
        for p in P:
            for s,x in L[p].items():
                o=[y for t,y in L[p].items() if t!=s]
                m=sum(o)/len(o)
                cv.append(x if m<0 else 0.0)
                cvs.append(x if all(y<math.log(0.8) for y in o) else 0.0)
        import random
        rnd=random.Random(1); bs_cv=[]; bs_t=[]
        cvp={p:[] for p in P}; tp={p:list(L[p].values()) for p in P}
        for p in P:
            for s,x in L[p].items():
                o=[y for t,y in L[p].items() if t!=s]
                cvp[p].append(x if sum(o)/len(o)<0 else 0.0)
        for _ in range(400):
            Q=[rnd.choice(P) for _ in P]
            bs_cv.append(sgm_ratio([x for q in Q for x in cvp[q]]))
        bs_cv.sort()
        lo,hi=bs_cv[10],bs_cv[389]
        out.append(dict(lo=lo,hi=hi,dir=d0,cfg=c,base=base,n=len(P),ns=len(S),tot=tot,fast=fast,slow=slow,rho=rho,orc=sgm_ratio(orc),cv=sgm_ratio(cv),cvs=sgm_ratio(cvs),L=L))
pickle.dump(out,open(DIR+"scan.pkl","wb"))
out.sort(key=lambda r:r["cv"])
print(f"{'dir':42s} {'cfg':14s} base  n ns  total  orc   cv  [95%]       cvS  rho  fast slow")
for r in out:
    print(f"{r['dir'][9:51]:42s} {r['cfg'][:14]:14s} {r['base'][:4]:4s} {r['n']:3d} {r['ns']:2d} {r['tot']:.3f} {r['orc']:.3f} {r['cv']:.3f} [{r['lo']:.3f},{r['hi']:.3f}] {r['cvs']:.3f} {r['rho']:5.2f} {len(r['fast']):3d} {len(r['slow']):3d}")
