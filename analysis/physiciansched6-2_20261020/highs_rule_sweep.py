import highspy, sys, time
# HiGHS の presolve_rule_off を 1 ビットずつ立て、前処理後の大きさと LP 緩和の値を出す
# usage: highs_rule_sweep.py <mps> [mask ...]   (mask を省くと 0 と 1<<6 .. 1<<19)
masks=[int(a) for a in sys.argv[2:]] or [0]+[1<<b for b in range(6,20)]
for mask in masks:
    h=highspy.Highs(); h.setOptionValue("threads",4); h.setOptionValue("output_flag",False)
    h.setOptionValue("presolve_rule_off",mask)
    h.readModel(sys.argv[1]); t=time.time(); h.presolve(); tp=time.time()-t
    lp=h.getPresolvedLp()
    nint=sum(1 for v in lp.integrality_ if v==highspy.HighsVarType.kInteger) if len(lp.integrality_) else 0
    g=highspy.Highs(); g.setOptionValue("output_flag",False); g.setOptionValue("threads",4)
    g.passModel(lp)
    g.setOptionValue("solve_relaxation",True); g.setOptionValue("presolve","off"); g.run()
    print(f"mask={mask:>7} bit={mask.bit_length()-1:>2} rows={lp.num_row_:>6} cols={lp.num_col_:>6} int={nint:>6} presolve={tp:5.1f}s lp={g.getInfo().objective_function_value:.4f}", flush=True)
