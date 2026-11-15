import highspy, sys
h=highspy.Highs(); h.setOptionValue("output_flag",False); h.readModel(sys.argv[1])
lp=h.getLp(); lp.integrality_=[]; h.passModel(lp); h.setOptionValue("presolve","off"); h.run()
print(sys.argv[1].split("/")[-1], "rows", lp.num_row_, "cols", lp.num_col_, "LP", h.getInfo().objective_function_value)
