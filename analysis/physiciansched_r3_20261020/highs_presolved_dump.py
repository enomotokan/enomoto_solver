"""HiGHS の前処理後の問題を MPS に書く: python highs_presolved_dump.py <mps> <出力>"""
import sys, highspy
h = highspy.Highs(); h.setOptionValue("output_flag", False)
h.readModel(sys.argv[1]); h.presolve()
lp = h.getPresolvedLp()
g = highspy.Highs(); g.setOptionValue("output_flag", False); g.passModel(lp); g.writeModel(sys.argv[2])
print(lp.num_row_, lp.num_col_)
