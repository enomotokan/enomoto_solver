import sys, highspy
h = highspy.Highs(); h.setOptionValue("threads", 4); h.setOptionValue("time_limit", float(sys.argv[1]))
h.setOptionValue("output_flag", True)
import os
if os.environ.get("HCUT"): h.setOptionValue("objective_bound", float(os.environ["HCUT"]))
for kv in os.environ.get("HOPT","").split(","):
    if kv: k,v=kv.split("="); h.setOptionValue(k, type(h.getOptionValue(k)[1])(v) if False else (int(v) if v.lstrip("-").isdigit() else (float(v) if v.replace(".","",1).isdigit() else v)))
h.readModel("/home/user/enomoto_solver/.miplib_cache/neos-911970.mps"); h.run()
print("RESULT", h.getInfo().objective_function_value, h.getInfo().mip_dual_bound, h.getInfo().mip_node_count)
