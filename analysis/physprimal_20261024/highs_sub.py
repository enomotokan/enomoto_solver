"""ENOMOTO のサブ MIP の書き出し (ENOMOTO_MIP_SUBMIP_DUMP) を HiGHS で解く:
python highs_sub.py <mps> [秒] [前処理 on/off]。最後の行に状態・目的値・時間。"""
import sys, time
import highspy

f = sys.argv[1]
T = float(sys.argv[2]) if len(sys.argv) > 2 else 30.0
pre = sys.argv[3] if len(sys.argv) > 3 else "on"
h = highspy.Highs()
h.setOptionValue("output_flag", False)
h.setOptionValue("time_limit", T)
h.setOptionValue("threads", 1)
h.setOptionValue("presolve", pre)
h.readModel(f)
t = time.perf_counter()
h.run()
t = time.perf_counter() - t
info = h.getInfo()
st = h.modelStatusToString(h.getModelStatus())
print(f"{f} status {st} obj {info.objective_function_value if info.primal_solution_status else None} "
      f"bound {info.mip_dual_bound:.6g} nodes {info.mip_node_count} time {t:.2f}", flush=True)
