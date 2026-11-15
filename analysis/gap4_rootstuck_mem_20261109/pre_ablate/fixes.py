import highspy, sys
p=sys.argv[1]
h0=highspy.Highs(); h0.setOptionValue("output_flag",False); h0.readModel(p); lp0=h0.getLp()
nm={s:i for i,s in enumerate(lp0.col_names_)}
h=highspy.Highs(); h.setOptionValue("output_flag",False); h.readModel(p)
off=0
for b in range(6,20):
    if b!=15: off|=1<<b
h.setOptionValue("presolve_rule_off",off); h.presolve(); pl=h.getPresolvedLp()
kept=set(nm[s] for s in pl.col_names_)
# 消えた列ごとに、2 回の後処理 (presolved 側を全部 0 / 全部 1 に近い値) で値が動かないなら固定とみなす
import numpy as np
vals=[]
for fill in (0.0,1.0):
    sol=highspy.HighsSolution(); sol.col_value=[min(max(fill,pl.col_lower_[k]),pl.col_upper_[k]) for k in range(pl.num_col_)]; sol.value_valid=True
    sol.row_value=[0.0]*pl.num_row_
    st=h.postsolve(sol)
    vals.append(list(h.getSolution().col_value))
rem=[j for j in range(lp0.num_col_) if j not in kept and lp0.col_lower_[j]<lp0.col_upper_[j]]
fixed=[(j,vals[0][j]) for j in rem if abs(vals[0][j]-vals[1][j])<1e-9]
print("removed",len(rem),"look fixed",len(fixed),"(to 0:",sum(1 for _,v in fixed if v<0.5),")")
lp1=h0.getLp(); lo=list(lp1.col_lower_); up=list(lp1.col_upper_)
for j,v in fixed: lo[j]=up[j]=round(v)
lp1.col_lower_=lo; lp1.col_upper_=up; lp1.integrality_=[]
g=highspy.Highs(); g.setOptionValue("output_flag",False); g.passModel(lp1); g.setOptionValue("presolve","off"); g.run()
print("p1 LP with HiGHS's fixings only:", g.getInfo().objective_function_value)
