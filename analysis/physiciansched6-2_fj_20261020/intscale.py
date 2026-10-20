import sys
# intscale.py <in> <out>: 整数列だけの行を、係数がすべて整数になる最小の倍率 (1..64) で拡大する
src,dst=sys.argv[1:3]
lines=open(src).read().split('\n')
sense={}; rows={}; integ=set(); sec=None; intm=False
for l in lines:
    if not l: continue
    if l[0]!=' ': sec=l.split()[0]; continue
    t=l.split()
    if sec=='ROWS': sense[t[1]]=t[0]; rows[t[1]]=[]
    elif sec=='COLUMNS':
        if len(t)>2 and t[1]=="'MARKER'": intm=t[2]=="'INTORG'"; continue
        if intm: integ.add(t[0])
        for k in range(1,len(t),2):
            if t[k] in rows: rows[t[k]].append((t[0],float(t[k+1])))
scale={}
for r,v in rows.items():
    if sense[r]=='N' or not v or not all(c in integ for c,_ in v): continue
    for k in range(1,65):
        if all(abs(a*k-round(a*k))<1e-9 for _,a in v):
            if k>1: scale[r]=k
            break
out=[]; sec=None
for l in lines:
    if l and l[0]!=' ': sec=l.split()[0]; out.append(l); continue
    t=l.split()
    if sec in ('COLUMNS','RHS','RANGES') and t and not (len(t)>2 and t[1]=="'MARKER'"):
        parts=[t[0]]
        for k in range(1,len(t),2):
            v=float(t[k+1])*scale.get(t[k],1)
            parts += [t[k], repr(v)]
        out.append("    "+"  ".join(parts)); continue
    out.append(l)
open(dst,'w').write('\n'.join(out))
from collections import Counter
print(len(scale),"rows scaled", Counter(scale.values()))
