import highspy, sys
# HiGHS の前処理後の問題を MPS に書き出す: highs_presolved_dump.py <mps> <out.mps> [key=value ...]
h=highspy.Highs(); h.setOptionValue("threads",4); h.setOptionValue("output_flag",False)
for kv in sys.argv[3:]:
    k,v=kv.split("=",1)
    try: v=int(v)
    except ValueError: pass
    h.setOptionValue(k,v)
h.readModel(sys.argv[1]); h.presolve()
lp=h.getPresolvedLp()
print("presolved", lp.num_row_, lp.num_col_)
h2=highspy.Highs(); h2.setOptionValue("output_flag",False); h2.passModel(lp); h2.writeModel(sys.argv[2])
