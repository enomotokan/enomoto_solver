# HIGHS_LPCUT_DUMP のファイルに、最初のループ (ブロック 1..999) のすべての行の和集合をブロック R 2000 として足す
import sys
fn=sys.argv[1]; seen={}; cur=None; first=True
lines=open(fn).read().splitlines()
for l in lines:
    if l.startswith('R '):
        k=int(l.split()[1])
        if k>=1000: break
        if cur is not None and k<=cur: break
        cur=k
    elif l.startswith('C ') and cur is not None:
        seen.setdefault(l,None)
with open(fn,'a') as f:
    f.write(f"R 2000 {len(seen)} 0\n")
    for l in seen: f.write(l+"\n")
print(len(seen))
