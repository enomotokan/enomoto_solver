import sys, highspy
name, T = sys.argv[1], float(sys.argv[2])
h = highspy.Highs()
h.setOptionValue("threads", 4); h.setOptionValue("time_limit", T); h.setOptionValue("output_flag", True)
for kv in sys.argv[3:]:
    k,v = kv.split("="); 
    try: v = int(v)
    except: 
        try: v=float(v)
        except: pass
    h.setOptionValue(k, v)
h.readModel(f"/home/user/enomoto_solver/.miplib_cache/{name}.mps")
h.run()
info=h.getInfo()
print("RESULT", name, h.getRunTime(), info.objective_function_value, info.mip_dual_bound, info.mip_node_count)
sol = h.getSolution().col_value
import os
out = os.environ.get("SOLOUT")
if out and info.objective_function_value < 1e20:
    open(out,"w").write("\n".join(repr(float(v)) for v in sol)+"\n")
