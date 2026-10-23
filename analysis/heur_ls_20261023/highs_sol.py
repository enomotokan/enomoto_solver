"""HiGHS で書き出した問題を解き、解を 1 行 1 値で書く: python -I highs_sol.py <mps> <秒> <出力>"""
import sys, highspy
h = highspy.Highs(); h.setOptionValue("time_limit", float(sys.argv[2])); h.setOptionValue("threads", 4)
h.readModel(sys.argv[1]); h.run()
info = h.getInfo(); print(h.modelStatusToString(h.getModelStatus()), info.objective_function_value, info.mip_dual_bound)
open(sys.argv[3], "w").write("".join(f"{v:.17e}\n" for v in h.getSolution().col_value))
