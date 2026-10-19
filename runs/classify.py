import highspy, numpy as np, collections, sys
h=highspy.Highs(); h.setOptionValue('output_flag',False); h.readModel(sys.argv[1])
lp=h.getLp(); A=lp.a_matrix_
st=np.array(A.start_); ix=np.array(A.index_); v=np.array(A.value_)
n=lp.num_col_; m=lp.num_row_
isint=np.array([int(x)!=0 for x in lp.integrality_])
rl=np.array(lp.row_lower_); ru=np.array(lp.row_upper_)
rows=[[] for _ in range(m)]
for j in range(n):
  for k in range(st[j],st[j+1]): rows[ix[k]].append((j,v[k]))
cnt=collections.Counter()
for i in range(m):
  r=rows[i]; nc=sum(1 for j,a in r if not isint[j]); ni=len(r)-nc
  allone=all(abs(a)==1 for j,a in r if isint[j])
  typ=('E' if rl[i]==ru[i] else ('L' if rl[i]<-1e20 else ('G' if ru[i]>1e20 else 'R')))
  cnt[(typ,'cont%d'%min(nc,2),'int'+('<=3' if ni<=3 else ('<=30' if ni<=30 else '>30')),'pm1' if allone else 'gen')]+=1
for k,c in sorted(cnt.items(), key=lambda x:-x[1]): print(c,k)
cl=np.array(lp.col_lower_); cu=np.array(lp.col_upper_)
print('cont bounds', list(zip(cl[~isint][:8].round(1), cu[~isint][:8].round(1))))
