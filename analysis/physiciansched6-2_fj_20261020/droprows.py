import sys
# droprows.py <in> <out> <rowsfile|minus:other.mps>: 行を除いた MPS を書く
src,dst,spec=sys.argv[1:4]
def rownames(f):
    s=set(); sec=None
    for l in open(f):
        if l[0]!=' ': sec=l.split()[0]; continue
        if sec=='ROWS': s.add(l.split()[1])
    return s
drop = rownames(src)-rownames(spec[6:]) if spec.startswith('minus:') else set(open(spec).read().split())
out=open(dst,'w'); sec=None
for l in open(src):
    if l[0]!=' ': sec=l.split()[0]; out.write(l); continue
    t=l.split()
    if sec=='ROWS' and t[1] in drop: continue
    if sec=='COLUMNS' and not (len(t)>2 and t[1]=="'MARKER'"):
        pairs=[(t[k],t[k+1]) for k in range(1,len(t),2) if t[k] not in drop]
        for r,v in pairs: out.write(f"    {t[0]}  {r}  {v}\n")
        continue
    if sec in ('RHS','RANGES'):
        pairs=[(t[k],t[k+1]) for k in range(1,len(t),2) if t[k] not in drop]
        for r,v in pairs: out.write(f"    {t[0]}  {r}  {v}\n")
        continue
    out.write(l)
print(len(drop),"rows dropped")
