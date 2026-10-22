import sys, highspy, numpy as np
for f in sys.argv[1:]:
    h=highspy.Highs(); h.setOptionValue('output_flag',False); h.readModel(f); lp=h.getLp(); n=lp.num_col_
    integ=[int(t) for t in lp.integrality_] if len(lp.integrality_) else [0]*n
    bad=[(j,lp.col_lower_[j],lp.col_upper_[j]) for j in range(n) if integ[j] and any(np.isfinite(v) and abs(v-round(v))>1e-12 for v in (lp.col_lower_[j],lp.col_upper_[j]))]
    print(f, 'ints', sum(integ), 'int cols with fractional bounds', len(bad), bad[:5])
