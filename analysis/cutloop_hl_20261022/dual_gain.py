# 同じ LP (HiGHS のラウンド k の後) で、候補のカットごとに 1 回目の双対単体の枢軸での目的値の伸び (違反量 x 最小比) を計算する
import sys, highspy, numpy as np
mps, cutsfile, k, candfile = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
def block(fn, want):
    out=[];cur=None
    for l in open(fn):
        t=l.split()
        if t[0]=='R': cur=int(t[1]); continue
        if t[0]=='C' and (want is None or cur==want):
            n=int(t[3]); out.append((float(t[1]) if t[1]!='-inf' else -np.inf, float(t[2]), [int(t[4+2*i]) for i in range(n)], [float(t[5+2*i]) for i in range(n)], float(t[t.index('E')+1]) if 'E' in t else 0))
    return out
h=highspy.Highs(); h.setOptionValue('output_flag',False); h.readModel(mps)
nc=h.getLp().num_col_
h.changeColsIntegrality(nc, np.arange(nc,dtype=np.int32), np.zeros(nc,dtype=np.uint8))
for c in block(cutsfile,k): h.addRow(c[0] if np.isfinite(c[0]) else -highspy.kHighsInf, c[1], len(c[2]), np.array(c[2],dtype=np.int32), np.array(c[3]))
h.run(); lp=h.getLp(); m=lp.num_row_
sol=h.getSolution(); x=np.array(sol.col_value); dcol=np.array(sol.col_dual); drow=np.array(sol.row_dual)
basis=h.getBasis(); bv=h.getBasicVariables()[1]   # >=0: column, <0: row -(i+1)
cs=basis.col_status; rs=basis.row_status
B=highspy.HighsBasisStatus.kBasic
Lb=highspy.HighsBasisStatus.kLower; Ub=highspy.HighsBasisStatus.kUpper
pos={}
for s,v in enumerate(bv): pos[v]=s
cache={}
def trow(s):
    if s not in cache:
        r=h.getReducedRow(s); cache[s]=np.array(r[1])  # B^-1 A row s (columns only)
    return cache[s]
def brow(s):
    return np.array(h.getBasisInverseRow(s)[1])
for c in block(candfile,None):
    j=np.array(c[2]); a=np.array(c[3]); viol=a@x[j]-c[1]
    alpha=np.zeros(nc); alpha_r=np.zeros(m)
    for jj,aa in zip(j,a):
        if cs[jj]==B:
            s=pos[jj]; alpha-=aa*trow(s); alpha_r-=aa*brow(s)  # 行のスラック (r_i = a_i x) の係数: -(B^-1)_s,i ... 符号は下で扱う
        else: alpha[jj]+=aa
    # 双対の比: 違反を減らす向きに動ける非基底 j (下限にあれば alpha_j<0、上限にあれば alpha_j>0) の |d_j/alpha_j| の最小
    best=np.inf
    for jj in range(nc):
        if cs[jj]==B or abs(alpha[jj])<1e-9: continue
        if (cs[jj]==Lb and alpha[jj]<0) or (cs[jj]==Ub and alpha[jj]>0) or (cs[jj]==highspy.HighsBasisStatus.kZero):
            best=min(best,abs(dcol[jj]/alpha[jj]))
    for i in range(m):
        if rs[i]==B or abs(alpha_r[i])<1e-9: continue
        # 行の活動量 r_i の係数は alpha_r (B^-1 の列 i に対応、HiGHS は行の論理変数 = +r_i)
        if (rs[i]==Lb and alpha_r[i]<0) or (rs[i]==Ub and alpha_r[i]>0):
            best=min(best,abs(drow[i]/alpha_r[i]))
    print(f"eff {c[4]:.3f} nnz {len(j):2d} viol {viol:.3f} firstpivot gain {viol*best if np.isfinite(best) else float('inf'):.3f}")
