import sys
# rowq.py <mps> row...: 行の中身を出す
f=sys.argv[1]; want=sys.argv[2:]
rows={}; sense={}; rhs={}; rng={}; integ=set(); sec=None; intm=False
for line in open(f):
    if line[0]!=' ': sec=line.split()[0]; continue
    t=line.split()
    if sec=='ROWS': sense[t[1]]=t[0]; rows[t[1]]=[]
    elif sec=='COLUMNS':
        if len(t)>2 and t[1]=="'MARKER'": intm = t[2]=="'INTORG'"; continue
        if intm: integ.add(t[0])
        for k in range(1,len(t),2):
            if t[k] in rows: rows[t[k]].append((t[0],float(t[k+1])))
    elif sec=='RHS':
        for k in range(1,len(t),2): rhs[t[k]]=float(t[k+1])
    elif sec=='RANGES':
        for k in range(1,len(t),2): rng[t[k]]=float(t[k+1])
for r in want:
    rr=rows[r]; print(f"{r} {sense[r]} {rhs.get(r,0)} rng={rng.get(r)} len={len(rr)}: "+" ".join(f"{v:+g}{x}{'i' if x in integ else ''}" for x,v in rr[:30]))
