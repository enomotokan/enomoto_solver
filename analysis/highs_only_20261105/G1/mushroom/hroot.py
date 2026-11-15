import highspy, sys
h = highspy.Highs()
h.readModel(sys.argv[1])
h.setOptionValue("threads", 4)
h.setOptionValue("mip_max_nodes", int(sys.argv[2]) if len(sys.argv)>2 else 1)
h.setOptionValue("time_limit", 120.0)
if len(sys.argv)>3: h.setOptionValue("objective_bound", float(sys.argv[3]))
h.run()
print("BOUND", h.getInfo().mip_dual_bound, h.getInfo().objective_function_value)
