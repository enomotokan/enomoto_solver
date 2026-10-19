import highspy, numpy as np, collections
h=highspy.Highs(); h.setOptionValue("output_flag",False); h.readModel("/home/user/enomoto_solver/.miplib_cache/neos-2978193-inde.mps")
lp=h.getLp(); print("rows",lp.num_row_,"cols",lp.num_col_,"nnz",len(lp.a_matrix_.index_))
it=np.array(lp.integrality_) if lp.integrality_ else None
print("integrality", collections.Counter([int(x) for x in lp.integrality_]))
cl=np.array(lp.col_lower_); cu=np.array(lp.col_upper_); c=np.array(lp.col_cost_)
print("bounds lo",collections.Counter(cl.round(3)).most_common(5),"up",collections.Counter(cu.round(3)).most_common(5))
print("cost nz", (c!=0).sum(), collections.Counter(c.round(4)).most_common(8))
ints=[j for j in range(lp.num_col_) if lp.integrality_[j]==1]
print("int cost", collections.Counter(c[ints].round(4)).most_common(5), "int bounds", collections.Counter(zip(cl[ints],cu[ints])).most_common(5))
rl=np.array(lp.row_lower_); ru=np.array(lp.row_upper_)
print("row lo", collections.Counter(rl.round(3)).most_common(5),"row up",collections.Counter(ru.round(3)).most_common(5))
A=lp.a_matrix_; st=np.array(A.start_); idx=np.array(A.index_); val=np.array(A.value_)
print("coef vals", collections.Counter(val.round(4)).most_common(10))
rowlen=np.bincount(idx,minlength=lp.num_row_); print("row len", collections.Counter(rowlen).most_common(10))
collen=np.diff(st); print("col len", collections.Counter(collen).most_common(10))
