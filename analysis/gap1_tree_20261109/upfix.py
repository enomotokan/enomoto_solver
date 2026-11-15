import highspy, numpy as np, time
U=-237.756681484
h=highspy.Highs(); h.setOptionValue("output_flag",False); h.setOptionValue("threads",1)
h.readModel("../../.miplib_cache/neos-957323.mps"); lp=h.getLp(); n=lp.num_col_
for j in range(n): h.changeColIntegrality(j, highspy.HighsVarType.kContinuous)
lo=np.array(lp.col_lower_); up=np.array(lp.col_upper_)
h.run(); t0=time.time()
for rnd in range(6):
    z=h.getInfo().objective_function_value; x=np.array(h.getSolution().col_value)
    f=x-np.floor(x); fr=np.where((f>1e-6)&(f<1-1e-6))[0]
    print("round",rnd,"LP %.7f"%z,"nfrac",len(fr),"t %.0f"%(time.time()-t0),flush=True)
    basis=h.getBasis(); fixed=[]
    for j in fr:
        h.changeColBounds(int(j),1.0,1.0); h.run()
        st=h.getModelStatus(); zz=h.getInfo().objective_function_value
        if st==highspy.HighsModelStatus.kInfeasible or zz>U: fixed.append(int(j))
        h.changeColBounds(int(j),lo[j],up[j]); h.setBasis(basis)
    print("  up-children pruned:",len(fixed),flush=True)
    if not fixed: break
    for j in fixed: up[j]=0; h.changeColBounds(j,0.0,0.0)
    h.run()
