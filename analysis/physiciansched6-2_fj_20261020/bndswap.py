import sys; sys.path.insert(0, sys.path[0])
import mpsmodel as M
# bndswap.py <src.mps> <dst.mps> <out.mps>: dst の列の境界を、両方にある連続列について src の境界にする
A=M.read(sys.argv[1]); B=M.read(sys.argv[2]); inf=float('inf'); n=0
for c in B['cols']:
    if c in A['bnd'] or c in A['cols']:
        if c not in B['integ'] and A['bnd'].get(c,(0.0,inf))!=B['bnd'].get(c,(0.0,inf)) and B['bnd'].get(c,(0,inf))[0]!=B['bnd'].get(c,(0,inf))[1]:
            B['bnd'][c]=A['bnd'].get(c,(0.0,inf)); n+=1
M.write(B,sys.argv[3]); print(n,"bounds changed")
