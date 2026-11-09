import highspy, numpy as np, time
def lpval(extra):
    h=highspy.Highs(); h.setOptionValue("output_flag",False); h.setOptionValue("threads",1)
    h.readModel(".miplib_cache/neos-957323.mps")
    lp=h.getLp(); n=lp.num_col_
    # LP relaxation
    for j in range(n): h.changeColIntegrality(j, highspy.HighsVarType.kContinuous)
    if extra:
        idx=np.array([j for j in range(n) if lp.col_cost_[j]!=0],dtype=np.int32)
        h.addRow(extra, highspy.kHighsInf, len(idx), idx, np.ones(len(idx)))
    t=time.time(); h.run(); return h.getInfo().objective_function_value, time.time()-t
print("LP", lpval(0))
print("LP + count>=238", lpval(238))
