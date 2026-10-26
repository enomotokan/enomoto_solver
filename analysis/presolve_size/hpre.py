"""HiGHS の前処理だけをかけて前処理後の問題を書き出す。使い方: python3 hpre.py <mps> <出力の mps>"""
import sys, time, highspy
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 1)
h.readModel(sys.argv[1]); t = time.time(); st = h.presolve(); t = time.time() - t
lp = h.getPresolvedLp()
print(f"HSTAT status={h.getModelPresolveStatus()} t={t:.2f} offset={lp.offset_}")
h.writePresolvedModel(sys.argv[2])
