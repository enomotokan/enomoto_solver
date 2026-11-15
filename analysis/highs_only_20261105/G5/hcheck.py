#!/usr/bin/env python3
"""縮約後の問題 (ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3 の MPS) に「目的値 <= 上限」を足して HiGHS で解き、上限以下の解があるかを見る。
使い方: hcheck.py MPS 上限 秒"""
import sys, highspy, numpy as np
h = highspy.Highs(); h.setOptionValue("threads", 4); h.setOptionValue("time_limit", float(sys.argv[3]))
h.readModel(sys.argv[1])
lp = h.getLp(); c = np.array(lp.col_cost_); nz = np.nonzero(c)[0]
off = lp.offset_
h.addRow(-highspy.kHighsInf, float(sys.argv[2]) - off, len(nz), nz.astype(np.int32), c[nz])
h.run()
print("status", h.getModelStatus(), "obj", h.getInfo().objective_function_value, "bound", h.getInfo().mip_dual_bound, file=sys.stderr)
