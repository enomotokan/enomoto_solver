import highspy, sys, collections
import numpy as np
h=highspy.Highs(); h.setOptionValue("output_flag",False); h.readModel(sys.argv[1])
lp=h.getLp()
n=lp.num_col_; m=lp.num_row_
lo=np.array(lp.col_lower_); up=np.array(lp.col_upper_); c=np.array(lp.col_cost_)
integ=[int(t) for t in lp.integrality_] if len(lp.integrality_) else [0]*n
A=lp.a_matrix_; st=np.array(A.start_); idx=np.array(A.index_); val=np.array(A.value_)
rows=collections.defaultdict(list)
for j in range(n):
    if lo[j]==up[j]: continue
    for k in range(st[j],st[j+1]): rows[idx[k]].append((j,val[k]))
free=[j for j in range(n) if lo[j]<up[j]]
print("free cols",len(free), "int", sum(1 for j in free if integ[j]), "bin", sum(1 for j in free if integ[j] and lo[j]==0 and up[j]==1))
print("free col bounds sample", collections.Counter((lo[j],up[j],integ[j]) for j in free).most_common(8))
print("cost of free", collections.Counter(np.sign(c[free])).most_common())
rl=np.array(lp.row_lower_); ru=np.array(lp.row_upper_)
pat=collections.Counter()
for i in range(m):
    r=rows.get(i,[])
    kinds=tuple(sorted(collections.Counter(('b' if integ[j] and lo[j]==0 and up[j]==1 else 'i' if integ[j] else 'c') for j,_ in r).items()))
    pat[(kinds if len(r)<6 else ('len>=6',)), rl[i]>-1e30, ru[i]<1e30]+=1
for k,v in pat.most_common(15): print(v,k)
# show a few long rows
import random
random.seed(1)
long=[i for i in range(m) if len(rows.get(i,[]))>=6]
print("long rows",len(long))
for i in long[:3]+random.sample(long,3):
    r=rows[i]
    print(i, rl[i], ru[i], len(r), [(j,round(a,4),integ[j],lo[j],up[j]) for j,a in r[:12]])
print("---")
cnt=collections.Counter()
for i in long:
    r=rows[i]
    pure=all(integ[j] for j,_ in r); ic=all(abs(a-round(a))<1e-9 for _,a in r)
    cnt[(pure,ic,rl[i]>-1e30, ru[i] if ru[i]<1e30 else None)]+=1
for k,v in cnt.most_common(10): print(v,k)
