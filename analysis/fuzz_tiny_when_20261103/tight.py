"""fuzz_tiny の seed を HiGHS で許容誤差 1e-6 (既定) と 1e-10 で解き、ENOMOTO の値と並べる。"""
import sys, highspy
sys.argv_extra = sys.argv[1:]; sys.argv = ["fuzz_tiny.py", "0"]
R = "/home/user/enomoto_solver/benchmarks/equiv2_bug_20261022/fuzz_tiny.py"
src = open(R).read().split("N = int(sys.argv[1])")[0]
ns = {"__file__": R}; exec(compile(src, R, "exec"), ns)
def hs(lp, tol):
    lo,up,c,it,rows,rlo,rup=lp; K=highspy.kHighsInf
    h=highspy.Highs(); h.setOptionValue('output_flag',False); h.setOptionValue('mip_rel_gap',1e-9)
    if tol: 
        for o in ('primal_feasibility_tolerance','mip_feasibility_tolerance'): h.setOptionValue(o, tol)
        h.setOptionValue('presolve','off')
    for j in range(len(lo)):
        h.addVar(lo[j], up[j]); h.changeColCost(j,c[j])
        if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r,l,u in zip(rows,rlo,rup): h.addRow(l if l>-1e300 else -K,u,len(r),[a for a,_ in r],[b for _,b in r])
    h.run(); return str(h.getModelStatus()).split('.')[-1], h.getInfo().objective_function_value
for s in map(int, sys.argv_extra):
    lp = ns["gen"](s); a = hs(lp, None); b = hs(lp, 1e-10); o = ns["ours"](lp)
    print(s, "highs(1e-6)", a, "highs(1e-10,presolve off)", b, "enomoto", o[0], o[1], flush=True)
