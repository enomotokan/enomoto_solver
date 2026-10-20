"""HiGHS で解いて解を 1 行 1 値で書く: python highs_sol.py <mps> <秒> <出力>"""
import sys, highspy
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 4)
h.setOptionValue("time_limit", float(sys.argv[2])); h.readModel(sys.argv[1]); h.run()
x = h.getSolution().col_value
open(sys.argv[3], "w").write("\n".join(repr(v) for v in x) + "\n")
print(h.getInfo().objective_function_value, len(x))
