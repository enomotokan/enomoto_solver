"""HiGHS 1.15.1 を開発者向けのログ (log_dev_level) つきで解く (G4 の調べ、2026-11-05)。
使い方: python -I hdev.py MPS 秒 [log_dev_level] > ログ"""
import sys, highspy
mps, tl = sys.argv[1], float(sys.argv[2])
lv = int(sys.argv[3]) if len(sys.argv) > 3 else 1
h = highspy.Highs()
h.setOptionValue("time_limit", tl); h.setOptionValue("threads", 4); h.setOptionValue("mip_rel_gap", 0.0)
h.setOptionValue("log_dev_level", lv)
h.readModel(mps); h.run()
print(h.getInfo().mip_dual_bound, h.getInfo().objective_function_value)
