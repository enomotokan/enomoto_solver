import sys; sys.path.insert(0, sys.path[0])
import mpsmodel as M
# hybrid2.py <A.mps> <B.mps> <out.mps> <B の消す行,..> <A の足す行,..>: B を土台に、指定した行を A の行に入れ替える
# (足す行の列の境界・費用・整数性は A のものにする。A の行の列のうち費用のある列の費用も A に合わせる)
A=M.read(sys.argv[1]); B=M.read(sys.argv[2]); dropB=[r for r in sys.argv[4].split(',') if r]; addA=[r for r in sys.argv[5].split(',') if r]
cols=set()
for r in dropB: del B['rows'][r]; B['order'].remove(r)
for r in addA:
    nm='A_'+r; B['rows'][nm]=[dict(A['rows'][r][0]),A['rows'][r][1],A['rows'][r][2]]; B['order'].append(nm); cols|=set(A['rows'][r][0])
for c in cols:
    if c not in B['cols']: B['cols'].append(c)
    B['bnd'][c]=A['bnd'].get(c,(0.0,float('inf')))
    B['cost'][c]=A['cost'].get(c,0.0)
    if c in A['integ']: B['integ'].add(c)
M.write(B,sys.argv[3]); print("ok",len(cols))
