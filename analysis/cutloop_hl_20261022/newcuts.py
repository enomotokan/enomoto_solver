# LPCUT_DUMP のブロック k+1 にあってブロック k にない行 (ラウンド k+1 で入ったカット) を INJECT 形式で書き出す
import sys
fn,k,out=sys.argv[1],int(sys.argv[2]),sys.argv[3]
B={};cur=None
for l in open(fn):
    if l.startswith('R '):
        kk=int(l.split()[1])
        if kk in B: break
        cur=kk; B[cur]=[]
    else: B[cur].append(l.rstrip())
old=set(B[k]); new=[l for l in B[k+1] if l not in old]
with open(out,'w') as f:
    f.write(f"R {k+1} {len(new)} 0\n"); f.write("\n".join(new)+"\n")
print(len(new))
