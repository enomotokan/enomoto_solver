"""HiGHS で MPS を解く: python highs_run.py <mps> <秒> [スレッド] [cutoff]。最後の行に JSON。"""
import json, sys, time
import highspy
mps, T = sys.argv[1], float(sys.argv[2])
th = int(sys.argv[3]) if len(sys.argv) > 3 else 1
h = highspy.Highs()
h.setOptionValue("time_limit", T)
h.setOptionValue("threads", th)
h.setOptionValue("mip_rel_gap", 1e-4)
if len(sys.argv) > 4:
    h.setOptionValue("objective_bound", float(sys.argv[4]))
h.readModel(mps)
t = time.perf_counter()
h.run()
t = time.perf_counter() - t
info = h.getInfo()
print(json.dumps({"status": h.modelStatusToString(h.getModelStatus()), "objective": info.objective_function_value,
                  "bound": info.mip_dual_bound, "nodes": info.mip_node_count, "time": t}), flush=True)
