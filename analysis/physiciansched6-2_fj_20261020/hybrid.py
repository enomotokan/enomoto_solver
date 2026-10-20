import sys; sys.path.insert(0, sys.path[0])
import mpsmodel as M
# hybrid.py <A.mps> <B.mps> <out.mps> <mode>: B を土台に A の要素を戻す。mode=scale: 列の集合が同じで倍率だけ違う行を A の形に、
# mode=chain: 片方にだけある行を A のものに入れ替え、その行の列の境界・費用も A のものに
A=M.read(sys.argv[1]); B=M.read(sys.argv[2]); mode=sys.argv[4]
key=lambda d: frozenset(d)
Akeys={}
for r,(d,lo,up) in A['rows'].items(): Akeys.setdefault(key(d),[]).append(r)
Bkeys={}
for r,(d,lo,up) in B['rows'].items(): Bkeys.setdefault(key(d),[]).append(r)
def same(x,y): return x[0]==y[0] and x[1]==y[1] and x[2]==y[2]
n=0
if mode=='scale':
    for k,rs in Bkeys.items():
        if k in Akeys and len(rs)==1 and len(Akeys[k])==1:
            a=A['rows'][Akeys[k][0]]; b=B['rows'][rs[0]]
            if not same(a,b): B['rows'][rs[0]]=[dict(a[0]),a[1],a[2]]; n+=1
elif mode=='chain':
    onlyB=[r for k,rs in Bkeys.items() if k not in Akeys for r in rs]
    onlyA=[r for k,rs in Akeys.items() if k not in Bkeys for r in rs]
    cols=set()
    for r in onlyB: cols|=set(B['rows'][r][0]); del B['rows'][r]; B['order'].remove(r)
    for r in onlyA:
        nm='A_'+r; B['rows'][nm]=[dict(A['rows'][r][0]),A['rows'][r][1],A['rows'][r][2]]; B['order'].append(nm); cols|=set(A['rows'][r][0])
    for c in cols:
        if c not in B['cols']: B['cols'].append(c)
        if c in A['bnd'] or c in A['cols']: B['bnd'][c]=A['bnd'].get(c,(0.0,float('inf')))
        B['cost'][c]=A['cost'].get(c,0.0)
        if c in A['integ']: B['integ'].add(c)
    n=(len(onlyB),len(onlyA))
M.write(B,sys.argv[3]); print(mode,n)
