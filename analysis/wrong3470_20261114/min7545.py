"""seed 7545 の最小の再現 (shrink_obj.py で d_0.mps から縮めた 8 列 3 行)。HiGHS と ENOMOTO の最適値を出す。"""
import highspy, io, contextlib
from enomoto_solver import Model, Variable
inf=float('inf')
cols={3:(-0.06123455040199044,2.0,False,-9.07),4:(0.0,2.0,True,-9.01),7:(-4.0,-2.0,True,3.94),8:(0.0,1.0,True,-0.19),9:(0.0,1.0,True,6.51),13:(-5.0,67.0,True,-4.11),15:(2.0,3.0,True,9.17),17:(0.0,5.0,True,-6.98)}
rows=[(24.46520723126155,[(3,2.0),(8,8.0),(9,-4.0),(15,8.0)],24.46520723126155),(-inf,[(4,7.0),(7,-3.0),(9,-3.0),(15,4.0),(17,3.0)],26.0),(-inf,[(j,1.0) for j in (3,4,7,8,9,13,15,17)],60.228789789242875)]
h=highspy.Highs(); h.setOptionValue('output_flag',False); h.setOptionValue('mip_rel_gap',1e-9); K=highspy.kHighsInf
pos={j:i for i,j in enumerate(cols)}
for j,(l,u,i,c) in cols.items():
    h.addVar(l,u); h.changeColCost(pos[j],c)
    if i: h.changeColIntegrality(pos[j],highspy.HighsVarType.kInteger)
for l,r,u in rows: h.addRow(max(l,-K),min(u,K),len(r),[pos[j] for j,_ in r],[v for _,v in r])
h.writeModel('min7545.mps'); h.run()
print("highs",h.modelStatusToString(h.getModelStatus()),h.getInfo().objective_function_value,dict(zip(cols,[round(v,6) for v in h.getSolution().col_value])))
M=Model(); xs={j:Variable(int if i else float,l,u) for j,(l,u,i,c) in cols.items()}
M.set_objective(sum(cols[j][3]*xs[j] for j in cols))
for l,r,u in rows:
    e=sum(v*xs[j] for j,v in r)
    if l==u: M.add_constraint(e==l)
    else: M.add_constraint(e<=u)
s=M.solve(mip_rel_gap=1e-9)
print("ours",s.status,s.objective,{j:xs[j].value for j in cols} if s.status=="optimal" else "")
