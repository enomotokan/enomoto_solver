# HiGHS を 4 スレッドで解く: python run_highs.py <mps> <T>
import sys, highspy
h = highspy.Highs(); h.setOptionValue("threads", 4); h.setOptionValue("mip_rel_gap", 0.0)
h.setOptionValue("time_limit", float(sys.argv[2])); h.setOptionValue("output_flag", True)
h.readModel(sys.argv[1]); h.run()
i = h.getInfo(); print("RESULT", h.modelStatusToString(h.getModelStatus()), i.objective_function_value, i.mip_dual_bound, i.mip_node_count)
