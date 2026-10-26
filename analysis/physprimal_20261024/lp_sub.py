"""サブ MIP の書き出しの LP 緩和を HiGHS で解く: python lp_sub.py <mps>"""
import sys, highspy, numpy as np
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1])
lp = h.getLp(); n = lp.num_col_
h.changeColsIntegrality(n, np.arange(n, dtype=np.int32), np.zeros(n, dtype=np.uint8))
h.run(); print(sys.argv[1], "LP", h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value)
