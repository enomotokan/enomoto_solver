# run_highs.py <MPS> <秒> <ログ>: HiGHS 1.15.1 を 4 スレッド・mip_rel_gap 0 で解き、ログを書く
import sys, highspy
mps, T, log = sys.argv[1], float(sys.argv[2]), sys.argv[3]
h = highspy.Highs()
h.setOptionValue("threads", 4)
h.setOptionValue("time_limit", T)
h.setOptionValue("mip_rel_gap", 0.0)
h.setOptionValue("log_file", log)
h.setOptionValue("output_flag", True)
h.readModel(mps)
h.run()
print(h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value, h.getRunTime())
