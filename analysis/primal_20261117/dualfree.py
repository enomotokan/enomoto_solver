"""sat_rens で HiGHS の isDualImpliedFree (≦ 行の双対の上限 ≤ 0 が連続列の双対の制約から 1 段で導ける) に当たる行の数を見積もる"""
import sys, highspy, numpy as np, collections
h=highspy.Highs(); h.setOptionValue("output_flag",False); h.readModel(sys.argv[1]); lp=h.getLp()
n,m=lp.num_col_,lp.num_row_
st=np.array(lp.a_matrix_.start_); idx=np.array(lp.a_matrix_.index_); val=np.array(lp.a_matrix_.value_)
lo=np.array(lp.col_lower_); up=np.array(lp.col_upper_); c=np.array(lp.col_cost_); it=[int(t) for t in lp.integrality_]
rl=np.array(lp.row_lower_); ru=np.array(lp.row_upper_)
rows=collections.defaultdict(list)
for j in range(n):
    for k in range(st[j],st[j+1]): rows[idx[k]].append((j,val[k]))
# 固定列を除いた行の活動の範囲
mn=np.zeros(m); mx=np.zeros(m); mni=np.zeros(m,int); mxi=np.zeros(m,int)
for i,r in rows.items():
    for j,a in r:
        l,u=(a*lo[j],a*up[j]) if a>0 else (a*up[j],a*lo[j])
        if np.isfinite(l): mn[i]+=l
        else: mni[i]+=1
        if np.isfinite(u): mx[i]+=u
        else: mxi[i]+=1
tol=1e-6
def implied(j, upper):
    # 列 j の上限 (upper) / 下限が、1 本の行から厳密に導けるか
    b = up[j] if upper else lo[j]
    if not np.isfinite(b): return True
    for k in range(st[j],st[j+1]):
        i=idx[k]; a=val[k]
        for side in ("u","l"):
            if side=="u" and not np.isfinite(ru[i]): continue
            if side=="l" and not np.isfinite(rl[i]): continue
            d = 1 if side=="u" else -1
            ad=d*a
            if (upper and ad<=0) or ((not upper) and ad>=0): continue
            act, inf = (mn[i],mni[i]) if d>0 else (-mx[i],mxi[i])
            own = ad*lo[j] if ad>0 else ad*up[j]
            if inf - (0 if np.isfinite(own) else 1) > 0: continue
            rest = act-own if np.isfinite(own) else act
            rhs = ru[i] if d>0 else -rl[i]
            imp=(rhs-rest)/ad
            if (upper and imp < b - tol) or ((not upper) and imp > b + tol): return True
    return False
# 行の双対の明示の境界 (HiGHS の符号: ≦ 行 y ≤ 0、≧ 行 y ≥ 0、等式・範囲は自由)
ylo=np.where(np.isfinite(ru)&~np.isfinite(rl), -np.inf, np.where(np.isfinite(rl)&~np.isfinite(ru), 0.0, -np.inf))
yhi=np.where(np.isfinite(ru)&~np.isfinite(rl), 0.0, np.where(np.isfinite(rl)&~np.isfinite(ru), np.inf, np.inf))
implU=np.full(m,np.inf); implL=np.full(m,-np.inf)
ncont=0; nfree=0
for j in range(n):
    if it[j]!=0 or lo[j]==up[j] or st[j]==st[j+1]: continue
    ncont+=1
    L = c[j] if implied(j,False) else -np.inf
    U = c[j] if implied(j,True) else np.inf
    if np.isfinite(L) and np.isfinite(U): nfree+=1
    ent=[(idx[k],val[k]) for k in range(st[j],st[j+1])]
    for i,a in ent:
        # 残りの和 Σ_{k≠i} a_k y_k の最小・最大
        smin=0.0; smax=0.0
        for k2,a2 in ent:
            if k2==i: continue
            l2,u2=(a2*ylo[k2],a2*yhi[k2]) if a2>0 else (a2*yhi[k2],a2*ylo[k2])
            smin+=l2; smax+=u2
        if np.isfinite(U) and np.isfinite(smin):
            b=(U-smin)/a
            if a>0: implU[i]=min(implU[i],b)
            else: implL[i]=max(implL[i],b)
        if np.isfinite(L) and np.isfinite(smax):
            b=(L-smax)/a
            if a>0: implL[i]=max(implL[i],b)
            else: implU[i]=min(implU[i],b)
le=np.isfinite(ru)&~np.isfinite(rl); ge=np.isfinite(rl)&~np.isfinite(ru)
dif = (le & (implU<=1e-7)) | (ge & (implL>=-1e-7))
print("連続列", ncont, "陰に自由な連続列", nfree, "≦行", le.sum(), "≧行", ge.sum(), "双対で陰に自由な不等式の行", int(dif.sum()))
