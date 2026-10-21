import sys, highspy
h = highspy.Highs(); h.readModel(sys.argv[1])
h.setOptionValue("threads",4); h.setOptionValue("mip_rel_gap",0.0); h.setOptionValue("time_limit",float(sys.argv[2])); h.setOptionValue("output_flag",True)
for kv in sys.argv[3:]:
    k,v=kv.split("=")
    try: h.setOptionValue(k, float(v))
    except Exception: h.setOptionValue(k, v=="true")
h.run(); print("RESULT", h.getInfo().objective_function_value, h.getInfo().mip_dual_bound, h.getInfo().mip_node_count)
