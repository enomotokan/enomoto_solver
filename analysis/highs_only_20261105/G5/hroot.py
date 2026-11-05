# neos-1456979: 行の種類ごとに行を消した問題を HiGHS で根だけ解き (1 スレッド・ノード 1 個)、根の下界を見る。
# 使い方: python hroot.py none|knap|bii|bcc  (repo の根で)
import highspy, sys, numpy as np, re, io, contextlib
mode=sys.argv[1]
h=highspy.Highs(); h.setOptionValue("output_flag",False)
h.readModel(".miplib_cache/neos-1456979.mps")
lp=h.getLp(); A=lp.a_matrix_; n=lp.num_col_; m=lp.num_row_
st=np.array(A.start_); idx=np.array(A.index_)
rl=np.zeros(m,int); kinds=[set() for _ in range(m)]
integ=[int(x) for x in lp.integrality_]
for j in range(n):
    for k in range(st[j],st[j+1]):
        i=idx[k]; rl[i]+=1
        kinds[i].add('B' if integ[j]==1 and lp.col_upper_[j]==1 else 'I' if integ[j]==1 else 'C')
drop=[]
for i in range(m):
    if mode=='knap' and rl[i]>500: drop.append(i)
    if mode=='bii' and kinds[i]=={'B','I'}: drop.append(i)
    if mode=='bcc' and kinds[i]=={'B','C'}: drop.append(i)
if drop: h.deleteRows(len(drop), np.array(drop,dtype=np.int32))
h.setOptionValue("mip_max_nodes",1); h.setOptionValue("threads",1); h.setOptionValue("time_limit",20.0)
h.setOptionValue("output_flag",True); h.setOptionValue("log_to_console",True)
h.run()
print(mode,"dropped",len(drop),"bound",h.getInfo().mip_dual_bound, "lp relax?")
