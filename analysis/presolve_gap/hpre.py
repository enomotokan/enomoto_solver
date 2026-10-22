# HiGHS の前処理だけをかけ、規則ごとの消去数を出し、前処理後の問題を書き出す
# 使い方: python3 hpre.py <mps> <出力の mps>
import sys, highspy
h = highspy.Highs()
h.setOptionValue("presolve_rule_logging", True)
h.setOptionValue("log_dev_level", 1)
h.setOptionValue("threads", 4)
h.readModel(sys.argv[1])
h.presolve()
lp = h.getPresolvedLp()
print("presolved rows", lp.num_row_, "cols", lp.num_col_, "nnz", lp.a_matrix_.index_.__len__())
h.writePresolvedModel(sys.argv[2])
