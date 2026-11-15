import highspy, sys
p = sys.argv[1]
def lp(presolve, relax=True, mip_presolve_only=False):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(p)
    if relax:
        lp_ = h.getLp(); lp_.integrality_ = []; h.passModel(lp_)
    h.setOptionValue("presolve", "on" if presolve else "off"); h.run()
    return h.getInfo().objective_function_value
print("LP relax (presolve off):", lp(False))
# HiGHS の MIP 前処理の後の問題の LP
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(p); h.presolve()
pl = h.getPresolvedLp()
print("presolved MIP size", pl.num_row_, pl.num_col_, "offset", pl.offset_)
g = highspy.Highs(); g.setOptionValue("output_flag", False); pl.integrality_ = []; g.passModel(pl); g.setOptionValue("presolve","off"); g.run()
print("LP of HiGHS-presolved MIP:", g.getInfo().objective_function_value)
