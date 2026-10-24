"""HiGHS をログつきで解く: python highs_log.py <mps> <秒> [スレッド] [キー=値 ...]。最後の行に JSON。"""
import json, sys, time
import highspy

mps, T = sys.argv[1], float(sys.argv[2])
th = int(sys.argv[3]) if len(sys.argv) > 3 else 4
h = highspy.Highs()
h.setOptionValue("time_limit", T)
h.setOptionValue("threads", th)
h.setOptionValue("mip_rel_gap", 0.0)
for kv in sys.argv[4:]:
    k, v = kv.split("=")
    for cast in (int, float, str):
        try:
            h.setOptionValue(k, cast(v)); break
        except Exception:
            pass
h.readModel(mps)
t = time.perf_counter()
h.run()
t = time.perf_counter() - t
info = h.getInfo()
print(json.dumps({"status": h.modelStatusToString(h.getModelStatus()), "objective": info.objective_function_value,
                  "bound": info.mip_dual_bound, "nodes": info.mip_node_count, "time": t}), flush=True)
