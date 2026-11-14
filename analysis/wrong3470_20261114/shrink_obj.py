"""dmp_q0.mps から行・列を落として、HiGHS と ENOMOTO の最適値が食い違ったままの最小の問題を探す。"""
import highspy, os, sys, io, contextlib
from enomoto_solver import Model, Variable
inf=float('inf')
def load(f):
    h=highspy.Highs(); h.setOptionValue('output_flag',False); h.readModel(f)
    lp=h.getLp(); n=lp.num_col_; m=lp.num_row_
    A=lp.a_matrix_; st=list(A.start_); idx=list(A.index_); val=list(A.value_)
    rows=[[] for _ in range(m)]
    for j in range(n):
        for p in range(st[j],st[j+1]): rows[idx[p]].append((j,val[p]))
    it=[t==highspy.HighsVarType.kInteger for t in lp.integrality_] if len(lp.integrality_) else [False]*n
    cl=lambda v: -inf if v<=-1e30 else (inf if v>=1e30 else v)
    return dict(lo=[cl(v) for v in lp.col_lower_],up=[cl(v) for v in lp.col_upper_],c=list(lp.col_cost_),it=it,rows=rows,rlo=[cl(v) for v in lp.row_lower_],rup=[cl(v) for v in lp.row_upper_])
def highs(P,cols,rws):
    h=highspy.Highs(); h.setOptionValue('output_flag',False); K=highspy.kHighsInf
    pos={j:i for i,j in enumerate(cols)}
    for j in cols:
        h.addVar(max(P['lo'][j],-K),min(P['up'][j],K)); h.changeColCost(pos[j],P['c'][j])
        if P['it'][j]: h.changeColIntegrality(pos[j],highspy.HighsVarType.kInteger)
    for i in rws:
        r=[(pos[j],v) for j,v in P['rows'][i] if j in pos]
        h.addRow(max(P['rlo'][i],-K),min(P['rup'][i],K),len(r),[a for a,_ in r],[b for _,b in r])
    h.run(); return (h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value)
def ours(P,cols,rws):
    M=Model(); xs={j:Variable(int if P['it'][j] else float,P['lo'][j],P['up'][j]) for j in cols}
    M.set_objective(sum(P['c'][j]*xs[j] for j in cols))
    for i in rws:
        r=[(j,v) for j,v in P['rows'][i] if j in xs]
        if not r: continue
        e=sum(v*xs[j] for j,v in r); l,u=P['rlo'][i],P['rup'][i]
        if l==u: M.add_constraint(e==l)
        else:
            if l>-inf: M.add_constraint(e>=l)
            if u<inf: M.add_constraint(e<=u)
    with contextlib.redirect_stdout(io.StringIO()):
        s=M.solve(mip_rel_gap=1e-9, log=False)
    return (s.status, s.objective)
P=load(sys.argv[1])
cols=list(range(len(P['lo']))); rws=list(range(len(P['rows'])))
def bad(c,r):
    hs,ho=highs(P,c,r)
    if hs!='Optimal': return False
    os_,oo=ours(P,c,r)
    return os_!='optimal' or abs(ho-oo)>1e-6*(1+abs(ho))
assert bad(cols,rws)
ch=True
while ch:
    ch=False
    for i in list(rws):
        r2=[x for x in rws if x!=i]
        if bad(cols,r2): rws=r2; ch=True
    for j in list(cols):
        c2=[x for x in cols if x!=j]
        if bad(c2,rws): cols=c2; ch=True
print("cols",cols,"rows",rws)
for j in cols: print(f"x{j}: [{P['lo'][j]},{P['up'][j]}] int={P['it'][j]} c={P['c'][j]}")
for i in rws: print(f"row{i}: {P['rlo'][i]} <= {' + '.join(f'{v!r}*x{j}' for j,v in P['rows'][i] if j in cols)} <= {P['rup'][i]}")
