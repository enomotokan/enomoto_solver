# HiGHS 1.15.1 を 4 スレッド・60 秒・mip_rel_gap 0 で解き、ログを標準出力に出す (physbound)
import sys, highspy
h = highspy.Highs()
h.setOptionValue("threads", 4); h.setOptionValue("time_limit", 60.0); h.setOptionValue("mip_rel_gap", 0.0)
h.readModel(sys.argv[1]); h.run()
print("STATUS", h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value, h.getInfo().mip_dual_bound)
