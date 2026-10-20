import sys
# rowset.py a.mps b.mps: 行を (向き, 右辺, 係数の組) で比べ、片方にだけある行を出す
def load(f):
    rows={}; sense={}; rhs={}; rng={}; sec=None; colname={}
    for line in open(f):
        if line[0]!=' ': sec=line.split()[0]; continue
        t=line.split()
        if sec=='ROWS': sense[t[1]]=t[0]; rows[t[1]]=[]
        elif sec=='COLUMNS':
            if len(t)>2 and t[1]=="'MARKER'": continue
            for k in range(1,len(t),2):
                if sense.get(t[k])!='N': rows[t[k]].append((t[0],round(float(t[k+1]),9)))
        elif sec=='RHS':
            for k in range(1,len(t),2): rhs[t[k]]=float(t[k+1])
        elif sec=='RANGES':
            for k in range(1,len(t),2): rng[t[k]]=float(t[k+1])
    return {(sense[r],rhs.get(r,0),rng.get(r),tuple(sorted(v))):r for r,v in rows.items() if sense[r]!='N'}
A=load(sys.argv[1]); B=load(sys.argv[2])
for k in A.keys()-B.keys(): print("A only", A[k], k[0],k[1],k[2], " ".join(f"{v:+g}{c}" for c,v in k[3][:20]), len(k[3]))
for k in B.keys()-A.keys(): print("B only", B[k], k[0],k[1],k[2], " ".join(f"{v:+g}{c}" for c,v in k[3][:20]), len(k[3]))
