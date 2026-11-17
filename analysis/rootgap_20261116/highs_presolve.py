"""HiGHS の前処理の規則ごとの減らし方を出す: python highs_presolve.py 問題 → logs/highs_presolve_<問題>.log"""
import sys, time, highspy
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
p = sys.argv[1]
h = highspy.Highs()
h.setOptionValue("output_flag", True); h.setOptionValue("log_to_console", False); h.setOptionValue("log_file", str(D / "logs" / f"highs_presolve_{p}.log"))
h.setOptionValue("presolve_rule_logging", True); h.setOptionValue("log_dev_level", 1); h.setOptionValue("threads", 4)
h.readModel(str(WT / ".miplib_cache" / f"{p}.mps"))
t = time.perf_counter(); h.presolve(); print(p, "presolve", round(time.perf_counter() - t, 1), "s")
lp = h.getPresolvedLp(); print(p, "rows", lp.num_row_, "cols", lp.num_col_, "nnz", len(lp.a_matrix_.value_))
