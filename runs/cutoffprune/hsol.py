# 使い方: python hsol.py <mps> <出力> : HiGHS (mip_rel_gap=0) で解き、列の値を 1 行 1 つで書く
import sys, highspy
h = highspy.Highs(); h.setOptionValue("mip_rel_gap", 0.0); h.setOptionValue("threads", 4)
h.readModel(sys.argv[1]); h.run()
print(h.getModelStatus(), h.getInfo().objective_function_value)
open(sys.argv[2], "w").write("\n".join(repr(v) for v in h.getSolution().col_value) + "\n")
