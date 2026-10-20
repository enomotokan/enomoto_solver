# 小さな MPS の読み書き (行: 名前 -> [係数 dict, lo, up]、列: 境界・費用・整数)
def read(f):
    rows={}; order=[]; sense={}; rhs={}; rng={}; cost={}; integ=set(); cols=[]; bnd={}; sec=None; intm=False; obj=None
    for line in open(f):
        if not line.strip(): continue
        if line[0]!=' ': sec=line.split()[0]; continue
        t=line.split()
        if sec=='ROWS':
            if t[0]=='N': obj=t[1]
            else: sense[t[1]]=t[0]; rows[t[1]]={}; order.append(t[1])
        elif sec=='COLUMNS':
            if len(t)>2 and t[1]=="'MARKER'": intm=t[2]=="'INTORG'"; continue
            c=t[0]
            if not cols or cols[-1]!=c: cols.append(c)
            if intm: integ.add(c)
            for k in range(1,len(t),2):
                if t[k]==obj: cost[c]=float(t[k+1])
                else: rows[t[k]][c]=float(t[k+1])
        elif sec=='RHS':
            for k in range(1,len(t),2):
                if t[k]!=obj: rhs[t[k]]=float(t[k+1])
        elif sec=='RANGES':
            for k in range(1,len(t),2): rng[t[k]]=float(t[k+1])
        elif sec=='BOUNDS':
            c=t[2]; lo,up=bnd.get(c,(0.0,float('inf')))
            v=float(t[3]) if len(t)>3 else None
            if t[0]=='UP': up=v
            elif t[0]=='LO': lo=v
            elif t[0]=='FX': lo=up=v
            elif t[0]=='MI': lo=float('-inf')
            elif t[0]=='PL': up=float('inf')
            elif t[0]=='BV': lo,up=0.0,1.0
            elif t[0]=='FR': lo,up=float('-inf'),float('inf')
            bnd[c]=(lo,up)
    R={}
    for r in order:
        b=rhs.get(r,0.0); s=sense[r]; g=rng.get(r)
        if s=='E': lo=up=b
        elif s=='L': up=b; lo=b-abs(g) if g is not None else float('-inf')
        else: lo=b; up=b+abs(g) if g is not None else float('inf')
        R[r]=[rows[r],lo,up]
    return dict(rows=R,order=order,cost=cost,integ=integ,cols=cols,bnd=bnd)
def write(m,f):
    inf=float('inf'); out=["NAME m","ROWS"," N OBJ"]
    col_rows={c:[] for c in m['cols']}
    for r in m['order']:
        d,lo,up=m['rows'][r]
        s='E' if lo==up else ('L' if up<inf else 'G')
        out.append(f" {s} {r}")
        for c,a in d.items(): col_rows.setdefault(c,[]).append((r,a))
    out.append("COLUMNS"); intm=False
    for c in m['cols']:
        isint=c in m['integ']
        if isint!=intm:
            out.append("    M  'MARKER'  " + ("'INTORG'" if isint else "'INTEND'")); intm=isint
        if m['cost'].get(c,0): out.append(f"    {c}  OBJ  {m['cost'][c]!r}")
        for r,a in col_rows.get(c,[]): out.append(f"    {c}  {r}  {a!r}")
    if intm: out.append("    M  'MARKER'  'INTEND'")
    out.append("RHS"); rg=[]
    for r in m['order']:
        d,lo,up=m['rows'][r]
        b=up if (lo==up or up<inf) else lo
        if b: out.append(f"    RHS  {r}  {b!r}")
        if lo!=up and lo>-inf and up<inf: rg.append((r,up-lo))
    if rg:
        out.append("RANGES"); out += [f"    RNG  {r}  {v!r}" for r,v in rg]
    out.append("BOUNDS")
    for c in m['cols']:
        lo,up=m['bnd'].get(c,(0.0,inf))
        if lo==up: out.append(f" FX BND {c} {lo!r}"); continue
        if lo==-inf: out.append(f" MI BND {c}")
        elif lo!=0: out.append(f" LO BND {c} {lo!r}")
        if up<inf: out.append(f" UP BND {c} {up!r}")
        elif c in m['integ']: out.append(f" PL BND {c}")
    out.append("ENDATA"); open(f,'w').write('\n'.join(out)+'\n')
