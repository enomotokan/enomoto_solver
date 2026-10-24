"""MPS の LP 緩和の値 (HiGHS、前処理なし): python lp_val.py <mps> ..."""
import sys, time, highspy
for f in sys.argv[1:]:
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f)
    h.setOptionValue("solve_relaxation", True); h.setOptionValue("presolve", "off")
    t = time.perf_counter(); h.run()
    lp = h.getLp()
    print(f, lp.num_row_, lp.num_col_, lp.a_matrix_.start_[-1], "offset", lp.offset_, "LP", h.getInfo().objective_function_value, "iters", h.getInfo().simplex_iteration_count, f"{time.perf_counter()-t:.2f}s")
