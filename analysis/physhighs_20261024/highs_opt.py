"""HiGHS をオプションつきで解き、ログ (各行に経過秒) を出す: python highs_opt.py <mps> <秒> [キー=値 ...]"""
import sys, time, highspy
mps, T = sys.argv[1], float(sys.argv[2])
h = highspy.Highs()
h.setOptionValue("time_limit", T); h.setOptionValue("threads", 4); h.setOptionValue("mip_rel_gap", 0.0)
for kv in sys.argv[3:]:
    k, v = kv.split("=")
    v = {"true": True, "false": False}.get(v, v)
    try: v = float(v) if isinstance(v, str) and "." in v else int(v) if isinstance(v, str) else v
    except ValueError: pass
    print(k, v, h.setOptionValue(k, v), flush=True)
h.readModel(mps)
t = time.perf_counter(); h.run()
print(f"RESULT {h.modelStatusToString(h.getModelStatus())} obj {h.getInfo().objective_function_value} bound {h.getInfo().mip_dual_bound} nodes {h.getInfo().mip_node_count} time {time.perf_counter() - t:.1f}", flush=True)
