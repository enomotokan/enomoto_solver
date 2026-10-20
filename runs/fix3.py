# 目的関数の列 x547・x644・x773 を 1 に固定した app1-1 を HiGHS (4 スレッド) で解き、実行可能解が出る時刻を見る
import highspy, numpy as np, sys, time
h=highspy.Highs(); h.setOptionValue("output_flag",True); h.setOptionValue("threads",4); h.setOptionValue("time_limit",60.0)
h.readModel(".miplib_cache/app1-1.mps")
lp=h.getLp(); names=list(lp.col_names_)
for nm in sys.argv[1:] or ["x547","x644","x773"]:
    h.changeColBounds(names.index(nm),1,1)
t=time.time(); h.run(); print("RESULT", time.time()-t, h.getModelStatus(), h.getInfo().objective_function_value, h.getInfo().mip_node_count)
