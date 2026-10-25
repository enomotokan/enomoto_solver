"""HiGHS の前処理を規則ごとの記録つきでかける (規則ごとに消した行・列の数)。使い方: python3 hlog.py <mps>"""
import sys, highspy
h = highspy.Highs(); h.setOptionValue("presolve_rule_logging", True); h.setOptionValue("log_dev_level", 1); h.setOptionValue("threads", 1)
h.readModel(sys.argv[1]); h.presolve()
