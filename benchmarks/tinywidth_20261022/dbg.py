import sys, os
seed=int(sys.argv[1]); sys.argv=[sys.argv[0],"0"]
src=open("fuzz_mixtiny.py").read().rsplit("N=int(sys.argv[1])",1)[0]
ns={"__file__":os.path.abspath("fuzz_mixtiny.py")}; exec(src,ns)
lp=ns["gen"](seed)
lo,up,c,it,rows,rlo,rup=lp
for j in range(len(lo)):
    if not it[j] and up[j]-lo[j]<1e-6: print("tiny col",j,repr(lo[j]),repr(up[j]),"cost",c[j])
for r,l,u in zip(rows,rlo,rup): print([(j,v,"I" if it[j] else "C") for j,v in r],l,u)
print(ns["highs"](lp), ns["ours"](lp)[:2])
