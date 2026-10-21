# 使い方: python hsol2.py <mps> <出力> [秒] : HiGHS (mip_rel_gap=0、時間上限) で解き、最適なら列の値を 1 行 1 つで書く
import sys, highspy
h = highspy.Highs(); h.setOptionValue("mip_rel_gap", 0.0); h.setOptionValue("threads", 2); h.setOptionValue("output_flag", False)
if len(sys.argv) > 3: h.setOptionValue("time_limit", float(sys.argv[3]))
h.readModel(sys.argv[1]); h.run()
st = h.getModelStatus(); print(sys.argv[1], st, h.getInfo().objective_function_value)
if st == highspy.HighsModelStatus.kOptimal:
    open(sys.argv[2], "w").write("\n".join(repr(v) for v in h.getSolution().col_value) + "\n")
