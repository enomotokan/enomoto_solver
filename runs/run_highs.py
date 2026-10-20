import sys, highspy
prob, T = sys.argv[1], float(sys.argv[2])
h=highspy.Highs(); h.setOptionValue("threads",4); h.setOptionValue("time_limit",T); h.setOptionValue("output_flag",True)
if len(sys.argv) > 3: h.setOptionValue("objective_bound", float(sys.argv[3]))  # 打ち切り値を与える (診断用)
h.readModel(f"/home/user/enomoto_solver/.miplib_cache/{prob}.mps"); h.run()
print("RESULT", prob, h.getInfo().objective_function_value, h.getInfo().mip_dual_bound, h.getInfo().mip_node_count, h.getRunTime())
