import sys, highspy
h=highspy.Highs(); h.setOptionValue("threads",int(sys.argv[3]) if len(sys.argv)>3 else 4); h.setOptionValue("time_limit",float(sys.argv[2]))
h.readModel(sys.argv[1]); h.run(); print("RESULT", h.getInfo().objective_function_value, h.getInfo().mip_node_count)
