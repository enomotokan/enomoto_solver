import highspy, sys
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 4)
h.readModel(sys.argv[1]); h.setOptionValue("mip_rel_gap", 0.0)
h.run(); print(h.getModelStatus(), h.getInfo().objective_function_value)
lp = h.getLp(); names = [lp.col_names_[j] for j in range(lp.num_col_)] if lp.col_names_ else None
x = h.getSolution().col_value
print("ncol", lp.num_col_, "first names", names[:3] if names else None)
open(sys.argv[2], "w").write("\n".join(repr(v) for v in x) + "\n")
