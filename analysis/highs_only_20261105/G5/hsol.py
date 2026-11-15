#!/usr/bin/env python3
"""HiGHS で解いて最適解を 1 行 1 値で書き出す (ENOMOTO_MIP_DEBUG_SOL 用)。使い方: hsol.py MPS 出力 秒"""
import sys, highspy
h = highspy.Highs(); h.setOptionValue("threads", 4); h.setOptionValue("time_limit", float(sys.argv[3]))
h.readModel(sys.argv[1]); h.run()
x = h.getSolution().col_value
print(h.getModelStatus(), h.getInfo().objective_function_value, file=sys.stderr)
open(sys.argv[2], "w").write("\n".join(repr(float(v)) for v in x) + "\n")
