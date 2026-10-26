# run_highs.py <mps> <秒> : HiGHS 1.15.1、4 スレッド、mip_rel_gap 0 でログを標準出力へ
import sys, highspy
h = highspy.Highs()
h.setOptionValue("threads", 4); h.setOptionValue("time_limit", float(sys.argv[2])); h.setOptionValue("mip_rel_gap", 0.0)
h.readModel(sys.argv[1]); h.run()
print("RESULT", h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value, h.getInfo().mip_dual_bound, h.getInfo().mip_node_count)
