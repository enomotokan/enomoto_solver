"""fuzz_tiny の問題を 0-1 列の全列挙で解く。連続列 (幅 <= 2e-9) は 0 に置く (目的への寄与は 300*2e-9 = 6e-7 以下)。
exact: 行を厳密に (違反 0) 満たす最良。tol: 違反 <= 1e-6 の最良 (連続列 0 のまま)。"""
import sys, itertools
F="/home/user/enomoto_solver/benchmarks/equiv2_bug_20261022/fuzz_tiny_cls.py"
src=open(F).read().split("cnt = {}")[0]
ns={"__file__":F}; exec(compile(src,F,"exec"),ns)
def best(lp, tol):
    lo,up,c,it,rows,rlo,rup=lp; nb=sum(it); bz=None
    for bits in itertools.product((0,1),repeat=nb):
        x=list(bits)+[0.0]*(len(lo)-nb)
        if all(l-tol <= sum(b*x[j] for j,b in r) <= u+tol for r,l,u in zip(rows,rlo,rup)):
            z=sum(cc*v for cc,v in zip(c,x)); bz=z if bz is None or z<bz else bz
    return bz
if __name__=="__main__":
    for s in map(int,sys.argv[1:]):
        lp=ns["gen"](s); print(s,"exact %.6g"%best(lp,0.0),"tol1e-6 %.6g"%best(lp,1e-6),"tol1.5e-9 %.6g"%best(lp,1.5e-9))
