import os
HERE = os.path.dirname(os.path.abspath(__file__)) + "/"
DIR = os.environ.get("REMINE_TMP", "/tmp/remine_20261118/")  # 中間の pickle (リポジトリに入れない)
os.makedirs(DIR, exist_ok=True)
# 実験 A で選んだ境目を、同じ環境変数の別の実験 B で評価する (実験をまたいだ検証)
import pickle, math, sys
SP=DIR
src=open(HERE+"stump.py").read()
exec(src.split("res=[]")[0])
def g(t): return next(r for r in out if r["dir"][9:]+":"+r["cfg"]==t)["L"]
def mean(L,p): return sum(L[p].values())/len(L[p])
for a,b in [(sys.argv[1],sys.argv[2]),(sys.argv[2],sys.argv[1])]:
    A,B=g(a),g(b)
    PA=[p for p in A if p in F]; rule=stump_fit(PA,{p:mean(A,p) for p in PA})
    PB=[p for p in B if p in F]
    allB=[x for p in PB for x in B[p].values()]
    selB=[(B[p][s] if sel(rule,p) else 0) for p in PB for s in B[p]]
    print(f"{a} で選ぶ: {rule[1]} {rule[3]} {rule[2]} → {b} で評価: 無条件 {math.exp(sum(allB)/len(allB)):.3f}  条件付き {math.exp(sum(selB)/len(selB)):.3f}  ({sum(sel(rule,p) for p in PB)}/{len(PB)} 問)")
