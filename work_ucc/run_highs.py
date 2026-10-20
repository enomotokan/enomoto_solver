"""HiGHS を 1 回解く: python run_highs.py <mps> <time_limit> [opt=val ...]"""
import sys, time, json
import highspy
mps, T = sys.argv[1], float(sys.argv[2])
h = highspy.Highs()
h.setOptionValue("time_limit", T)
h.setOptionValue("threads", 4)
for kv in sys.argv[3:]:
    k, v = kv.split("=")
    for conv in (int, float):
        try: v = conv(v); break
        except ValueError: pass
    if v in ("true", "false"): v = (v == "true")
    h.setOptionValue(k, v)
h.readModel(mps)
t = time.perf_counter(); h.run(); t = time.perf_counter() - t
info = h.getInfo()
print("RESULT " + json.dumps({"status": h.modelStatusToString(h.getModelStatus()), "objective": info.objective_function_value,
      "bound": info.mip_dual_bound, "nodes": info.mip_node_count, "time": t}), flush=True)
