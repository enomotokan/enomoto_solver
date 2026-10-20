import sys
# mpsq.py <mps> col...: 列の境界・費用・行 (行の全体) を出す
f=sys.argv[1]; want=set(sys.argv[2:])
rows={}; sense={}; cols={}; rhs={}; rng={}; bnd={}; cost={}; integ=set(); sec=None; intm=False
for line in open(f):
    if not line.strip(): continue
    if line[0]!=' ':
        sec=line.split()[0]; continue
    t=line.split()
    if sec=='ROWS': sense[t[1]]=t[0]; rows[t[1]]=[]
    elif sec=='COLUMNS':
        if t[1]=="'MARKER'": intm = t[2]=="'INTORG'"; continue
        c=t[0]
        if intm: integ.add(c)
        for k in range(1,len(t),2):
            r,v=t[k],float(t[k+1])
            if sense.get(r)=='N': cost[c]=v
            else: rows[r].append((c,v)); cols.setdefault(c,[]).append(r)
    elif sec=='RHS':
        for k in range(1,len(t),2): rhs[t[k]]=float(t[k+1])
    elif sec=='RANGES':
        for k in range(1,len(t),2): rng[t[k]]=float(t[k+1])
    elif sec=='BOUNDS':
        bnd.setdefault(t[2],{})[t[0]]=float(t[3]) if len(t)>3 else None
for c in want:
    print(f"## {c} int={c in integ} cost={cost.get(c,0)} bnd={bnd.get(c)}")
    for r in cols.get(c,[]):
        rr=rows[r]
        s=" ".join(f"{v:+g}{x}{'i' if x in integ else ''}" for x,v in rr[:40])
        print(f"  {r} {sense[r]} {rhs.get(r,0)} rng={rng.get(r)} len={len(rr)}: {s}")
