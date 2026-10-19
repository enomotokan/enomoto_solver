import sys, highspy
T=float(sys.argv[2]) if len(sys.argv)>2 else 60
h=highspy.Highs(); h.setOptionValue("output_flag",True); h.setOptionValue("threads",4); h.setOptionValue("time_limit",T)
for kv in sys.argv[3:]:
    k,v=kv.split("="); 
    try: v=int(v)
    except: 
        try: v=float(v)
        except: pass
    h.setOptionValue(k,v)
h.readModel(sys.argv[1]); h.run()
i=h.getInfo(); print("RESULT",h.getModelStatus(),i.objective_function_value,i.mip_dual_bound,i.mip_node_count)
