import highspy, sys
p=sys.argv[1]; keep=[int(x) for x in sys.argv[2].split(",")]
off=0
for b in range(6,20):
    if b not in keep: off|=1<<b
h=highspy.Highs(); h.setOptionValue("output_flag",True); h.setOptionValue("log_dev_level",1); h.readModel(p)
h.setOptionValue("presolve_rule_off",off); h.presolve()
pl=h.getPresolvedLp(); pl.integrality_=[]
g=highspy.Highs(); g.setOptionValue("output_flag",False); g.passModel(pl); g.setOptionValue("presolve","off"); g.run()
print("KEEP",keep,"LP",g.getInfo().objective_function_value + 0, "offset", pl.offset_, "size", pl.num_row_, pl.num_col_)
