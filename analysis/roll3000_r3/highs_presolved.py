# HiGHS の前処理後の問題を書き出し、LP 緩和の値を比べる
import highspy, sys
mps = sys.argv[1]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(mps)
h.presolve(); pm = h.getPresolvedLp()
h2 = highspy.Highs(); h2.setOptionValue("output_flag", False); h2.passModel(pm); h2.writeModel("/tmp/claude-0/roll3000_highs_pre.mps")
print("presolved", pm.num_row_, pm.num_col_)
for name, mdl in [("orig", None), ("pre", pm)]:
    g = highspy.Highs(); g.setOptionValue("output_flag", False)
    if mdl is None: g.readModel(mps)
    else: g.passModel(mdl)
    lp = g.getLp(); lp.integrality_ = []; g.passModel(lp); g.run()
    print(name, "LP relaxation", g.getInfo().objective_function_value)
# 前処理後の MIP を前処理なしで根だけ
g = highspy.Highs(); g.setOptionValue("presolve", "off"); g.setOptionValue("mip_max_nodes", 1); g.setOptionValue("threads", 4)
g.setOptionValue("output_flag", True); g.passModel(pm); g.run(); print("pre+presolve_off root dual bound", g.getInfo().mip_dual_bound)
