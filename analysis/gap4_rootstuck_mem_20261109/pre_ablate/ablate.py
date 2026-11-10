import highspy, sys
p=sys.argv[1]
def val(off):
    h=highspy.Highs(); h.setOptionValue("output_flag",False); h.readModel(p)
    h.setOptionValue("presolve_rule_off",off); h.presolve()
    pl=h.getPresolvedLp(); pl.integrality_=[]
    g=highspy.Highs(); g.setOptionValue("output_flag",False); g.passModel(pl); g.setOptionValue("presolve","off"); g.run()
    return g.getInfo().objective_function_value, pl.num_row_, pl.num_col_
names={6:"forcing row",7:"forcing col",8:"free col subst",9:"doubleton eq",10:"dep eq",11:"dep free col",12:"aggregator",13:"parallel",14:"sparsify",15:"probing",16:"enumeration",17:"dual fixing",18:"col stuffing",19:"initial sweep"}
print("all on", val(0))
for b,n in names.items(): print(n, val(1<<b))
print("probing+enumeration off", val((1<<15)|(1<<16)))
