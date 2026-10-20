import sys; sys.path.insert(0, sys.path[0])
import mpsmodel as M
# colcmp.py a.mps b.mps: 自由な列 (固定でない列) の費用・境界・整数性の違い
A=M.read(sys.argv[1]); B=M.read(sys.argv[2]); inf=float('inf')
def free(X):
    used=set(c for d,_,_ in X['rows'].values() for c in d)
    return {c for c in X['cols'] if c in used and X['bnd'].get(c,(0.0,inf))[0]!=X['bnd'].get(c,(0.0,inf))[1]}
fa,fb=free(A),free(B)
print("free A",len(fa),"B",len(fb),"onlyA",len(fa-fb),"onlyB",len(fb-fa))
print(" onlyA:",sorted(fa-fb)[:20]); print(" onlyB:",sorted(fb-fa)[:20])
from collections import Counter
cd=Counter(); bd=[]
for c in fa&fb:
    ca,cb=A['cost'].get(c,0.0),B['cost'].get(c,0.0)
    if abs(ca-cb)>1e-9*(1+abs(ca)): cd[(round(ca,6),round(cb,6))]+=1
    if A['bnd'].get(c,(0.0,inf))!=B['bnd'].get(c,(0.0,inf)): bd.append((c,A['bnd'].get(c,(0.0,inf)),B['bnd'].get(c,(0.0,inf))))
print("cost diffs (A,B):",cd.most_common(12), "total", sum(cd.values()))
print("bound diffs",len(bd),bd[:10])
