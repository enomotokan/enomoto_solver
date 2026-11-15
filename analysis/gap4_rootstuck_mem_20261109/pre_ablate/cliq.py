import highspy, sys
def stats(lp, label):
    import numpy as np
    m=lp.num_row_; a=lp.a_matrix_
    # 列圧縮を行へ
    rows=[[] for _ in range(m)]
    st=list(a.start_); ix=list(a.index_); vv=list(a.value_)
    for j in range(lp.num_col_):
        for k in range(st[j],st[j+1]): rows[ix[k]].append((j,vv[k]))
    integ=list(lp.integrality_) if len(lp.integrality_) else [None]*lp.num_col_
    isbin=[ (len(integ)>j and integ[j]==highspy.HighsVarType.kInteger and lp.col_lower_[j]==0 and lp.col_upper_[j]==1) for j in range(lp.num_col_)]
    n=0; tot=0; hist={}
    for i,r in enumerate(rows):
        if r and all(isbin[j] and abs(v-1)<1e-12 for j,v in r) and lp.row_upper_[i]<=1+1e-9 and lp.row_upper_[i]>=1-1e-9:
            n+=1; tot+=len(r); hist[len(r)]=hist.get(len(r),0)+1
        elif r and all(isbin[j] and abs(v+1)<1e-12 for j,v in r) and lp.row_lower_[i]>=-1-1e-9 and lp.row_lower_[i]<=-1+1e-9:
            n+=1; tot+=len(r); hist[len(r)]=hist.get(len(r),0)+1
    print(label, "rows", m, "set-packing rows", n, "nnz", tot, "len hist", sorted(hist.items())[:12], "...")
for p in sys.argv[1:]:
    h=highspy.Highs(); h.setOptionValue("output_flag",False); h.readModel(p); stats(h.getLp(), p.split("/")[-1])
h=highspy.Highs(); h.setOptionValue("output_flag",False); h.readModel(sys.argv[1])
off=0
for b in range(6,20):
    if b!=15: off|=1<<b
h.setOptionValue("presolve_rule_off",off); h.presolve(); stats(h.getPresolvedLp(), "highs(p1, probing only)")
