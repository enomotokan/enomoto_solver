# highs_cut.py <mps> <秒> <スレッド> <打ち切り値>: HiGHS に最適値の打ち切り値 (objective_bound) を与えて解く
import sys, highspy
h = highspy.Highs()
h.setOptionValue("time_limit", float(sys.argv[2])); h.setOptionValue("threads", int(sys.argv[3]))
h.setOptionValue("mip_rel_gap", 0.0); h.setOptionValue("objective_bound", float(sys.argv[4]))
h.readModel(sys.argv[1]); h.run()
print(h.getModelStatus(), h.getInfo().objective_function_value, h.getInfo().mip_dual_bound, h.getInfo().mip_node_count, h.getRunTime())
