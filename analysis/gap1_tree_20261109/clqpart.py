import highspy, numpy as np, collections, sys
name=sys.argv[1]
h=highspy.Highs(); h.setOptionValue("output_flag",False); h.readModel(f".miplib_cache/{name}.mps")
lp=h.getLp(); A=lp.a_matrix_
st=np.array(A.start_); idx=np.array(A.index_); val=np.array(A.value_)
c=np.array(lp.col_cost_); ru=np.array(lp.row_upper_); rl=np.array(lp.row_lower_)
n=lp.num_col_
rows=[[] for _ in range(lp.num_row_)]
for j in range(n):
    for k in range(st[j],st[j+1]): rows[idx[k]].append((j,val[k]))
# set packing rows: all coef 1 on binaries, rhs 1
pk=[i for i,r in enumerate(rows) if ru[i]==1 and all(v==1 for _,v in r)]
colrows=collections.defaultdict(set)
for i in pk:
    for j,_ in rows[i]: colrows[j].add(i)
obj=[j for j in range(n) if c[j]<0]
obj.sort(key=lambda j:c[j], reverse=True)  # HiGHS order: descending cost
assigned=set(); parts=[]
order=obj
pos={j:k for k,j in enumerate(order)}
remaining=list(order)
while remaining:
    v=remaining[0]; part=[v]; cand=[u for u in remaining[1:] if colrows[u]&colrows[v]]
    while cand:
        u=cand[0]; part.append(u); cand=[w for w in cand[1:] if colrows[w]&colrows[u]]
    ps=set(part); parts.append(part); remaining=[u for u in remaining if u not in ps]
lb=sum(min(c[j] for j in p) for p in parts)
print("packing rows",len(pk),"partitions",len(parts),"sizes",collections.Counter(len(p) for p in parts).most_common(8),"comb lb",lb)
