import highspy, numpy as np, itertools, time
h=highspy.Highs(); h.setOptionValue("output_flag",False); h.readModel(".miplib_cache/app1-1.mps")
lp=h.getLp(); n=lp.num_col_; c=np.array(lp.col_cost_)
obj=[j for j in range(n) if c[j]!=0]
# relax integrality
h.changeColsIntegrality(n, np.arange(n,dtype=np.int32), np.zeros(n,dtype=np.uint8))
h.run(); print("LP", h.getInfo().objective_function_value)
def feas(ones):
    for j in ones: h.changeColBounds(j,1,1)
    h.run(); st=h.getModelStatus(); v=h.getInfo().objective_function_value
    for j in ones: h.changeColBounds(j,0,1)
    return st==highspy.HighsModelStatus.kOptimal, v
alive=[]
for j in obj:
    f,v=feas([j]); print(lp.col_names_[j], f, round(v,3)); 
    if f: alive.append(j)
print("alive", len(alive))
pairs=[p for p in itertools.combinations(alive,2) if feas(p)[0]]
print("feasible pairs", len(pairs))
tr=[t for t in itertools.combinations(alive,3) if all(p in pairs or (p[1],p[0]) in pairs for p in itertools.combinations(t,2)) and feas(t)[0]]
print("LP-feasible triples", len(tr), [[lp.col_names_[j] for j in t] for t in tr[:20]])
