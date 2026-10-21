"""HiGHS 1.15.1 で 1 問を解く (4 スレッド)。使い方: python run_highs.py <mps> [秒] [opt=val ...]"""
import sys, highspy
h = highspy.Highs()
h.setOptionValue("threads", 4); h.setOptionValue("mip_rel_gap", 0.0); h.setOptionValue("output_flag", True)
h.setOptionValue("time_limit", float(sys.argv[2]) if len(sys.argv) > 2 else 60.0)
for kv in sys.argv[3:]:
    k, v = kv.split("=")
    for cast in (int, float, str):
        try: h.setOptionValue(k, cast(v)); break
        except Exception: pass
h.readModel(sys.argv[1]); h.run()
i = h.getInfo()
print("RESULT", h.getModelStatus(), i.objective_function_value, i.mip_dual_bound, i.mip_node_count, flush=True)
