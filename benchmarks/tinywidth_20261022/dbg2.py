import sys, os
seed=int(sys.argv[1]); sys.argv=[sys.argv[0],"0"]
p=os.path.abspath("../equiv2_bug_20261022/fuzz_tiny.py")
src=open(p).read().rsplit("N = int(sys.argv[1])",1)[0]
ns={"__file__":p}; exec(src,ns)
lp=ns["gen"](seed); lo,up,c,it,rows,rlo,rup=lp
print("up",up); print("c",c)
for r,l,u in zip(rows,rlo,rup): print(r,l,repr(u))
print(ns["highs"](lp), ns["ours"](lp)[:2])
