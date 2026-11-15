import highspy, numpy as np, time, sys
mode=sys.argv[1]  # up/down/first
h=highspy.Highs(); h.setOptionValue("output_flag",False); h.setOptionValue("threads",1)
h.readModel("../../.miplib_cache/neos-957323.mps"); lp=h.getLp(); n=lp.num_col_
for j in range(n): h.changeColIntegrality(j, highspy.HighsVarType.kContinuous)
h.run()
for it in range(25):
    z=h.getInfo().objective_function_value; st=h.getModelStatus()
    x=np.array(h.getSolution().col_value); f=x-np.floor(x); fr=np.where((f>1e-6)&(f<1-1e-6))[0]
    print(it, h.modelStatusToString(st), "%.7f"%z, len(fr), flush=True)
    if len(fr)==0 or st!=highspy.HighsModelStatus.kOptimal: break
    if mode=="first": j=fr[0]; v=0.0
    else:
        j=fr[np.argmax(f[fr]*(1-f[fr]))]; v=1.0 if mode=="up" else 0.0
    h.changeColBounds(int(j), v, v); h.run()
