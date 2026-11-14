"""python3 check_seed.py SEED: 元の fuzz の問題を両方で解き、返した解の元の問題での違反を出す。"""
import sys, os, io, contextlib, highspy
fz={}; exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)),'../../scripts/mip_fuzz_vs_highs.py')).read().split("N=int(sys.argv[1])")[0],fz)
inf=float('inf')
seed=int(sys.argv[1]); lp=fz['gen'](seed); lo,up,c,it,rows,rlo,rup=lp
def viol(x):
    v=0.0
    for j in range(len(lo)):
        v=max(v,lo[j]-x[j],x[j]-up[j]); 
        if it[j]: v=max(v,abs(x[j]-round(x[j])))
    for r,l,u in zip(rows,rlo,rup):
        a=sum(t*x[j] for j,t in r); v=max(v,l-a,a-u)
    return v
h=highspy.Highs(); h.setOptionValue('output_flag',False); h.setOptionValue('mip_rel_gap',1e-9); K=highspy.kHighsInf
for j in range(len(lo)):
    h.addVar(lo[j] if lo[j]>-inf else -K, up[j] if up[j]<inf else K); h.changeColCost(j,c[j])
    if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
for r,l,u in zip(rows,rlo,rup): h.addRow(l if l>-inf else -K,u if u<inf else K,len(r),[a for a,_ in r],[b for _,b in r])
h.writeModel(f"seed{seed}.mps"); h.run(); st=h.modelStatusToString(h.getModelStatus())
print("highs",st,h.getInfo().objective_function_value, "viol %.2e"%viol(list(h.getSolution().col_value)) if st=='Optimal' else "")
with contextlib.redirect_stdout(io.StringIO()): s,o,x=fz['ours'](lp)
print("ours",s,o,"viol %.2e"%viol(x) if x else "")
if x: print("x =",x)
