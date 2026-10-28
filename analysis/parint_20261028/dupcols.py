import sys, highspy, collections
for n in sys.argv[1:]:
    h=highspy.Highs(); h.setOptionValue('output_flag',False); h.readModel(f'.miplib_cache/{n}.mps')
    lp=h.getLp(); A=lp.a_matrix_; st,ix,va=A.start_,A.index_,A.value_
    integ=list(lp.integrality_) if len(lp.integrality_) else [0]*lp.num_col_
    g=collections.Counter(); gi=collections.Counter()
    for j in range(lp.num_col_):
        key=(lp.col_cost_[j],lp.col_lower_[j],lp.col_upper_[j],int(integ[j]),tuple(sorted(zip(ix[st[j]:st[j+1]],va[st[j]:st[j+1]]))))
        g[key]+=1
    dup_int=sum(c-1 for k,c in g.items() if c>1 and k[3]); dup_cont=sum(c-1 for k,c in g.items() if c>1 and not k[3])
    print(f"{n:20s} cols {lp.num_col_:7d}  removable duplicates: integer {dup_int:6d} continuous {dup_cont:5d}")
