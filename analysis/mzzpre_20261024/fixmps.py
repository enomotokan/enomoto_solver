"""元の問題に固定 (hdiff.py の tsv の fix) だけを当てた MPS を書く: python fixmps.py <mps> <tsv> <出力.mps>"""
import sys
import numpy as np
import highspy
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1])
lp = h.getLp(); n0 = {n: j for j, n in enumerate(lp.col_names_)}
lo = np.array(lp.col_lower_); up = np.array(lp.col_upper_); k = 0
for line in open(sys.argv[2]):
    nm, kind, v, _ = line.split("\t")
    if kind == "fix": j = n0[nm]; lo[j] = up[j] = round(float(v)); k += 1
lp.col_lower_ = lo; lp.col_upper_ = up
g = highspy.Highs(); g.setOptionValue("output_flag", False); g.passModel(lp); g.writeModel(sys.argv[3]); print("fixed", k)
