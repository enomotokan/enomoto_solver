"""HiGHS (highspy 1.15.1、1 スレッド) で書き出したサブ MIP を解く: python highs_dump.py <mps> [打ち切り値] [種]"""
import sys, highspy
h = highspy.Highs()
h.readModel(sys.argv[1])
h.setOptionValue("threads", 1)
h.setOptionValue("time_limit", 120.0)
if len(sys.argv) > 2: h.setOptionValue("objective_bound", float(sys.argv[2]))
if len(sys.argv) > 3: h.setOptionValue("random_seed", int(sys.argv[3]))
h.run()
print("RESULT", h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value, h.getInfo().mip_node_count, h.getRunTime())
