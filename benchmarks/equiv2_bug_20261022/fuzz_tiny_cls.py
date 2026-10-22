"""fuzz_tiny の食い違いを分ける: HiGHS の解の元の問題での最大違反 (厳密に実行可能か) と、こちらの値が良いか悪いか。"""
import sys, highspy, numpy as np
sys.argv = [sys.argv[0], "0"] + sys.argv[1:]
src = open(__file__.replace("fuzz_tiny_cls.py", "fuzz_tiny.py")).read().split("N = int(sys.argv[1])")[0]
ns = {"__file__": __file__.replace("fuzz_tiny_cls.py", "fuzz_tiny.py")}; exec(src, ns)
gen, ours = ns["gen"], ns["ours"]
def hsol(lp):
    lo,up,c,it,rows,rlo,rup=lp
    h=highspy.Highs(); h.setOptionValue('output_flag',False); h.setOptionValue('mip_rel_gap',1e-9); K=highspy.kHighsInf
    for j in range(len(lo)):
        h.addVar(lo[j], up[j]); h.changeColCost(j,c[j])
        if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r,l,u in zip(rows,rlo,rup): h.addRow(l if l>-1e300 else -K,u,len(r),[a for a,_ in r],[b for _,b in r])
    h.run(); return h.getInfo().objective_function_value, list(h.getSolution().col_value)
def viol(lp, x):
    lo,up,c,it,rows,rlo,rup=lp
    v=max([0]+[max(lo[j]-x[j],x[j]-up[j]) for j in range(len(x))])
    for r,l,u in zip(rows,rlo,rup):
        a=sum(b*x[j] for j,b in r); v=max(v,l-a,a-u)
    return v
cnt = {}
for seed in range(int(sys.argv[2]), int(sys.argv[2]) + int(sys.argv[3])):
    lp=gen(seed); ho,hx=hsol(lp); os_,oo,ox=ours(lp)
    if os_=="optimal" and abs(ho-oo)<=1e-6*(1+abs(ho)): continue
    hv=viol(lp,hx); ov=viol(lp,ox) if ox else None
    # HiGHS の解の整数列を丸め、連続列を 0 にした点 (厳密に実行可能か)
    xr=[round(v) if lp[3][j] else 0.0 for j,v in enumerate(hx)]; rv=viol(lp,xr); zr=sum(c*v for c,v in zip(lp[2],xr))
    kind = ("ours worse" if oo is None or oo > ho else "ours better")
    k = kind + (" / HiGHS-int point exactly feasible" if rv <= 0 else " / HiGHS-int point viol %.0e" % rv)
    cnt[k]=cnt.get(k,0)+1
    print(seed, kind, "highs", ho, "viol %.1e" % hv, "ours", os_, oo, "viol", ov if ov is None else "%.1e" % ov, "| int point obj", zr, "viol %.1e" % rv)
print(cnt)
