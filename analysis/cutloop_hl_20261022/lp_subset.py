# highspy で LP 緩和 + HiGHS のラウンド k の LP のカット + 追加のカットの集合ごとの LP 値 (どのカットが効くか)
import sys, highspy, numpy as np
mps, cutsfile, k, candfile = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
def block(fn, want):
    out=[];cur=None
    for l in open(fn):
        t=l.split()
        if t[0]=='R': cur=int(t[1]); continue
        if t[0]=='C' and (want is None or cur==want):
            n=int(t[3]); out.append((float(t[1]) if t[1]!='-inf' else -np.inf, float(t[2]), [int(t[4+2*i]) for i in range(n)], [float(t[5+2*i]) for i in range(n)], ' '.join(t[t.index('E'):]) if 'E' in t else ''))
    return out
h=highspy.Highs(); h.setOptionValue('output_flag',False); h.readModel(mps)
lp=h.getLp(); nc=lp.num_col_
h.changeColsIntegrality(nc, np.arange(nc,dtype=np.int32), np.zeros(nc,dtype=np.uint8))
def add(c): h.addRow(c[0] if np.isfinite(c[0]) else -highspy.kHighsInf, c[1], len(c[2]), np.array(c[2],dtype=np.int32), np.array(c[3]))
for c in block(cutsfile,k): add(c)
h.run(); base=h.getInfo().objective_function_value; print('base',base)
C=block(candfile,None); m0=h.getLp().num_row_
for i,c in enumerate(C):
    add(c); h.run(); v=h.getInfo().objective_function_value
    h.deleteRows(1,np.array([m0],dtype=np.int32)); print(f"{i:2d} alone +{v-base:8.3f} {c[4]} nnz {len(c[2])}")
for c in C: add(c)
h.run(); print('all', h.getInfo().objective_function_value)
