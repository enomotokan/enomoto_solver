# highs_log.py <mps> <秒> <スレッド>: HiGHS 1.15.1 を mip_rel_gap 0 でログ付きで解く
import sys, highspy
h = highspy.Highs()
h.setOptionValue("time_limit", float(sys.argv[2])); h.setOptionValue("threads", int(sys.argv[3]))
h.setOptionValue("mip_rel_gap", 0.0)
h.readModel(sys.argv[1]); h.run()
print(h.getModelStatus(), h.getInfo().objective_function_value, h.getInfo().mip_dual_bound, h.getInfo().mip_node_count)
