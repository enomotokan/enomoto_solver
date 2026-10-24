"""HiGHS の前処理後の問題を MPS に書く: python highs_presolved.py <mps> <出力.mps>。目的の定数項を表示する。"""
import sys, time, highspy
h = highspy.Highs()
h.setOptionValue("threads", 4)
h.readModel(sys.argv[1])
t = time.perf_counter(); h.presolve(); t = time.perf_counter() - t
lp = h.getPresolvedLp()
h2 = highspy.Highs(); h2.setOptionValue("output_flag", False)
h2.passModel(lp)
h2.writeModel(sys.argv[2])
print(f"presolve {t:.1f}s rows {lp.num_row_} cols {lp.num_col_} offset {lp.offset_}", flush=True)
