"""highspy で mzzv42z の根だけを解き (mip_max_nodes=1 相当: mip_max_leaves=1 ではなくノード上限 0)、根のカット後の下界を出す。
使い方: python run_highs_root.py <mps> <presolve on|off> [opt=val ...]"""
import sys, time, highspy
mps, pre = sys.argv[1], sys.argv[2]
h = highspy.Highs()
h.setOptionValue("output_flag", True)
h.setOptionValue("threads", 4)
h.setOptionValue("presolve", pre)
h.setOptionValue("mip_max_nodes", 1)
h.setOptionValue("mip_heuristic_effort", 0.0)
h.setOptionValue("time_limit", 120.0)
for kv in sys.argv[3:]:
    k, v = kv.split("=", 1)
    t = type(h.getOptionValue(k)[1])
    h.setOptionValue(k, t(v) if t is not bool else v in ("1", "true", "True"))
h.readModel(mps)
t = time.perf_counter(); h.run()
info = h.getInfo()
print("RESULT", pre, sys.argv[3:], info.mip_dual_bound, info.objective_function_value, info.mip_node_count, round(time.perf_counter() - t, 1), flush=True)
