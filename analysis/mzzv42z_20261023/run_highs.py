"""HiGHS で 1 問解く: python run_highs.py <mps> <秒> [スレッド]。ログを出し、最後の行に RESULT。"""
import sys, time, highspy
mps, T = sys.argv[1], float(sys.argv[2])
th = int(sys.argv[3]) if len(sys.argv) > 3 else 4
h = highspy.Highs()
h.setOptionValue("threads", th); h.setOptionValue("time_limit", T); h.setOptionValue("mip_rel_gap", 0.0)
h.readModel(mps)
t = time.perf_counter(); h.run(); t = time.perf_counter() - t
info = h.getInfo()
print(f"RESULT status={h.modelStatusToString(h.getModelStatus())} obj={info.objective_function_value} bound={info.mip_dual_bound} nodes={info.mip_node_count} time={t:.1f}", flush=True)
