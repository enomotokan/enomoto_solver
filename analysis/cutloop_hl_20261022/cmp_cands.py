# HiGHS が LP に入れたカット (INJECT 形式のブロック) ごとに、ENOMOTO の候補 (DEBUG_CAND_DUMP のブロック) の中で最も平行なものを探す
import sys, math
def blocks(fn):
    B={};cur=None
    for l in open(fn):
        t=l.split()
        if t[0]=='R': cur=int(t[1]); B[cur]=[]
        elif t[0]=='C':
            n=int(t[3]); d={int(t[4+2*k]):float(t[5+2*k]) for k in range(n)}
            B[cur].append((d,float(t[2])))
    return B
H=blocks(sys.argv[1])[int(sys.argv[3])]; E=blocks(sys.argv[2])[int(sys.argv[4])]
def norm(d): return math.sqrt(sum(v*v for v in d.values()))
cnt=0
for d,r in H:
    best=0;bj=None
    for e,re in E:
        dot=sum(v*e.get(j,0) for j,v in d.items())
        c=dot/(norm(d)*norm(e)+1e-300)
        if c>best: best=c;bj=(e,re)
    same = best>0.9999 and abs(r/norm(d)-bj[1]/norm(bj[0]))<1e-6
    weaker = best>0.9999 and not same
    cnt+=same
    print(f"nnz {len(d):3d} rhs {r:9.4g} best cos {best:.4f} {'SAME' if same else ('PARALLEL rhs ' + format(bj[1]/norm(bj[0])*norm(d),'.4g') if weaker else '')}")
print("same", cnt, "of", len(H))
