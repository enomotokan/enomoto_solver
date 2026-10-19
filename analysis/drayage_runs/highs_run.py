import sys, highspy
p, T = sys.argv[1], float(sys.argv[2])
h = highspy.Highs()
h.setOptionValue("output_flag", True)
h.setOptionValue("threads", 4)
h.setOptionValue("time_limit", T)
for kv in sys.argv[3:]:
    k, v = kv.split("=")
    try: v = int(v)
    except ValueError:
        try: v = float(v)
        except ValueError: v = {"true":True,"false":False}.get(v, v)
    h.setOptionValue(k, v)
h.readModel(p)
h.run()
print("OBJ", h.getInfo().objective_function_value, "BOUND", h.getInfo().mip_dual_bound, "NODES", h.getInfo().mip_node_count)
