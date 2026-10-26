"""MPS の行のうち、整数列の係数が 1e-6 未満の行を数える (前処理が作った小さな係数の行)"""
import sys, highspy, numpy as np, scipy.sparse as sp
for f in sys.argv[1:]:
    h=highspy.Highs(); h.setOptionValue('output_flag',False); h.readModel(f); lp=h.getLp(); n=lp.num_col_
    integ=np.array([int(t) for t in lp.integrality_]) if len(lp.integrality_) else np.zeros(n,int)
    A=lp.a_matrix_; M=sp.csc_matrix((A.value_,A.index_,A.start_),shape=(lp.num_row_,n)).tocoo()
    m=(np.abs(M.data)<1e-6)&(integ[M.col]!=0)
    rows=sorted(set(M.row[m].tolist()))
    print(f, 'rows with tiny int coefs', len(rows), rows[:8])
