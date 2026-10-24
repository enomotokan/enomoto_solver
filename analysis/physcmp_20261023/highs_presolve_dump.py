"""HiGHS の前処理だけをかけ、前処理後の問題を MPS に書き、その LP 緩和を解く。
python highs_presolve_dump.py <mps> <出力 mps> [キー=値 ...] (log_dev_level=1 で前処理の規則ごとの統計)"""
import sys, time, highspy
h = highspy.Highs(); h.setOptionValue("output_flag", True); h.setOptionValue("threads", 4)
for kv in sys.argv[3:]:
    k, v = kv.split("=")
    for cast in (int, float, str):
        try:
            if h.setOptionValue(k, cast(v)) == highspy.HighsStatus.kOk: break
        except Exception: pass
h.readModel(sys.argv[1]); t = time.perf_counter(); h.presolve(); tp = time.perf_counter() - t
lp = h.getPresolvedLp()
g = highspy.Highs(); g.setOptionValue("output_flag", False); g.passModel(lp); g.writeModel(sys.argv[2])
g.setOptionValue("solve_relaxation", True); g.setOptionValue("presolve", "off"); g.run()
print("PRESOLVED", lp.num_row_, lp.num_col_, lp.a_matrix_.start_[-1], "offset", lp.offset_, "time", round(tp, 2), "LP", g.getInfo().objective_function_value)
