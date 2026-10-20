import sys
# rowset2.py a.mps b.mps: 行を列の集合で突き合わせ、(1) 同じ行 (2) 倍率だけ違う行 (3) 列の集合は同じで係数が違う行 (4) 片方だけ、を数える
def load(f):
    rows={}; sense={}; rhs={}; rng={}; sec=None
    for line in open(f):
        if line[0]!=' ': sec=line.split()[0]; continue
        t=line.split()
        if sec=='ROWS': sense[t[1]]=t[0]; rows[t[1]]=[]
        elif sec=='COLUMNS':
            if len(t)>2 and t[1]=="'MARKER'": continue
            for k in range(1,len(t),2):
                if sense.get(t[k])!='N': rows[t[k]].append((t[0],float(t[k+1])))
        elif sec=='RHS':
            for k in range(1,len(t),2): rhs[t[k]]=float(t[k+1])
        elif sec=='RANGES':
            for k in range(1,len(t),2): rng[t[k]]=float(t[k+1])
    out={}
    for r,v in rows.items():
        if sense[r]=='N' or not v: continue
        b=rhs.get(r,0); s=sense[r]; g=rng.get(r)
        if s=='E': lo=up=b
        elif s=='L': up=b; lo=b-abs(g) if g is not None else float('-inf')
        else: lo=b; up=b+abs(g) if g is not None else float('inf')
        out.setdefault(frozenset(c for c,_ in v),[]).append((r,dict(v),lo,up))
    return out
def canon(d,lo,up):
    k=min(d); s=d[k]
    cs=tuple(sorted((c,round(a/s,9)) for c,a in d.items()))
    l,u=lo/s,up/s
    if s<0: l,u=u,l
    return cs,round(l,9),round(u,9)
A=load(sys.argv[1]); B=load(sys.argv[2])
same=scaled=coef=0; onlyA=[]; onlyB=[]; ex={'scaled':[], 'coef':[]}
for key,la in A.items():
    lb=B.get(key,[])
    used=set()
    for (r,d,lo,up) in la:
        m=None
        for t,(r2,d2,lo2,up2) in enumerate(lb):
            if t in used: continue
            if d==d2 and lo==lo2 and up==up2: m=('same',t); break
        if m is None:
            for t,(r2,d2,lo2,up2) in enumerate(lb):
                if t in used: continue
                if canon(d,lo,up)==canon(d2,lo2,up2): m=('scaled',t); break
        if m is None:
            for t,(r2,d2,lo2,up2) in enumerate(lb):
                if t in used: continue
                m=('coef',t); break
        if m is None: onlyA.append((r,d,lo,up)); continue
        used.add(m[1])
        if m[0]=='same': same+=1
        elif m[0]=='scaled': scaled+=1; ex['scaled'].append(((r,d,lo,up),lb[m[1]]))
        else: coef+=1; ex['coef'].append(((r,d,lo,up),lb[m[1]]))
    for t,x in enumerate(lb):
        if t not in used: onlyB.append(x)
for key,lb in B.items():
    if key not in A: onlyB.extend(lb)
print(f"same {same} scaled {scaled} coef {coef} onlyA {len(onlyA)} onlyB {len(onlyB)}")
fmt=lambda x: f"{x[0]} [{x[2]},{x[3]}] len {len(x[1])}: "+" ".join(f"{a:+g}{c}" for c,a in sorted(x[1].items())[:12])
for k in ['scaled','coef']:
    for a,b in ex[k][:int(sys.argv[3]) if len(sys.argv)>3 else 4]: print(k,"A",fmt(a)); print(k,"B",fmt(b))
for x in onlyA[:int(sys.argv[3]) if len(sys.argv)>3 else 4]: print("onlyA",fmt(x))
for x in onlyB[:int(sys.argv[3]) if len(sys.argv)>3 else 4]: print("onlyB",fmt(x))
