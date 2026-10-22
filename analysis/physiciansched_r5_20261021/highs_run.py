"""HiGHS 1.15.1 を 1 回解く: python highs_run.py <mps> <秒> [option=値 ...] (ログつき、4 スレッド、mip_rel_gap=0)。"""
import sys, time, highspy
h = highspy.Highs()
h.setOptionValue("time_limit", float(sys.argv[2]))
h.setOptionValue("threads", 4)
h.setOptionValue("mip_rel_gap", 0.0)
for kv in sys.argv[3:]:
    k, v = kv.split("=", 1)
    for conv in (int, float, str):
        try:
            h.setOptionValue(k, conv(v)); break
        except Exception:
            continue
h.readModel(sys.argv[1])
t = time.perf_counter(); h.run(); t = time.perf_counter() - t
info = h.getInfo()
print("RESULT", h.modelStatusToString(h.getModelStatus()), info.objective_function_value, info.mip_dual_bound,
      info.mip_node_count, round(t, 2), flush=True)
