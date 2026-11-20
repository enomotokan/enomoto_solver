import os
HERE = os.path.dirname(os.path.abspath(__file__)) + "/"
DIR = os.environ.get("REMINE_TMP", "/tmp/remine_20261118/")  # 中間の pickle (リポジトリに入れない)
os.makedirs(DIR, exist_ok=True)
import pickle, math, sys
exec(open(HERE+"stump.py").read().split("out=pickle.load")[0])
out=pickle.load(open(SP+"scan.pkl","rb"))
NOISY=("noswot","neos-911970","neos-787933","fiball","CMS750_4","rococoC10","kasai","timtab1","mc11","pk1","10teams")
for w in sys.argv[1:]:
    tg,feat,dr,thr=w.split("|"); thr=float(thr)
    r=next(r for r in out if (r["dir"][9:]+":"+r["cfg"])==tg)
    L=r["L"]
    sel=[p for p in L if feat in F.get(p,{}) and ((F[p][feat]>thr) if dr==">" else (F[p][feat]<=thr))]
    lr=[x for p in sel for x in L[p].values()]; allr=[x for p in L for x in L[p].values()]
    print(f"== {tg}  {feat}{dr}{thr}: 選ぶ {len(sel)}/{len(L)} 問  選んだ問題の比 {math.exp(sum(lr)/max(1,len(lr))):.3f}  全体に入れた形 {math.exp(sum(lr)/len(allr)):.3f} (無条件 {r['tot']:.3f})")
    for p in sorted(sel,key=lambda p:sum(L[p].values())/len(L[p])):
        v=L[p]; print(f"   {p:24s} {'揺' if p.startswith(NOISY) else ' '} {feat}={F[p][feat]:.3g}  比 "+" ".join(f"{math.exp(x):.2f}" for x in v.values()))
    print("   stable fast:",r["fast"]," slow:",r["slow"])
