import collections
f="/home/user/enomoto_solver/.miplib_cache/neos-2978193-inde.mps"
sec=None; rows={}; cols=collections.OrderedDict(); rhs={}; bnd=[]; intm=False; ints=set()
for line in open(f):
    if not line.startswith(" "):
        sec=line.split()[0]; continue
    t=line.split()
    if sec=="ROWS": rows[t[1]]=t[0]
    elif sec=="COLUMNS":
        if "'MARKER'" in t: intm = "'INTORG'" in t; continue
        c=t[0]
        if intm: ints.add(c)
        d=cols.setdefault(c,{})
        for i in range(1,len(t),2): d[t[i]]=float(t[i+1])
    elif sec=="RHS":
        for i in range(1,len(t),2): rhs[t[i]]=float(t[i+1])
    elif sec=="BOUNDS": bnd.append(t)
obj=[r for r,k in rows.items() if k=="N"][0]
print("rows",collections.Counter(rows.values()))
print("rhs",collections.Counter((rows[r],v) for r,v in rhs.items()).most_common(10))
print("bounds",collections.Counter(b[0] for b in bnd), bnd[:3])
il=list(ints)[:3]
for c in sorted(ints)[:3]: print(c, cols[c])
cont=[c for c in cols if c not in ints]
for c in cont[:3]: print(c, cols[c])
# row membership
rowcols=collections.defaultdict(list)
for c,d in cols.items():
    for r,v in d.items():
        if r!=obj: rowcols[r].append((c,v))
for r in list(rows)[:2]+list(rows)[64:66]+list(rows)[-3:]:
    if r in rowcols: print(r, rows[r], len(rowcols[r]), rowcols[r][:6], rhs.get(r))
print("L-row lens", collections.Counter(len(rowcols[r]) for r,k in rows.items() if k=="L"))
print("E-row lens", collections.Counter(len(rowcols[r]) for r,k in rows.items() if k=="E"))
print("E row coefs", collections.Counter(v for r,k in rows.items() if k=="E" for c,v in rowcols[r]).most_common(5))
print("L row coefs", collections.Counter(round(v,3) for r,k in rows.items() if k=="L" for c,v in rowcols[r]).most_common(5))
