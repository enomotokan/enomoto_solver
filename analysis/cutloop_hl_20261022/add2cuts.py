# HIGHS_ADD_DUMP (A 回 右辺 長さ 列 係数 ...) を INJECT 用の形 (R 回 / C 下限 上限 長さ ...) に直す
import sys
out=open(sys.argv[2],'w'); cur=None
for l in open(sys.argv[1]):
    t=l.split()
    if t[0]!='A': continue
    if t[1]!=cur: cur=t[1]; out.write(f"R {cur} 0 0\n")
    out.write("C -inf "+" ".join(t[2:])+"\n")
