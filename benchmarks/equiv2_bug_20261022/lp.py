import sys, highspy, numpy as np
off = float(sys.argv[1])
for f in sys.argv[2:]:
    h=highspy.Highs(); h.setOptionValue('output_flag',False); h.readModel(f)
    lp=h.getLp(); n=lp.num_col_
    h.changeColsIntegrality(n, np.arange(n,dtype=np.int32), np.zeros(n,dtype=np.uint8)); h.run()
    print(f, n, lp.num_row_, sum(lp.col_lower_[j]<lp.col_upper_[j] for j in range(n)), h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value+off, flush=True)
