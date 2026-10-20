"""HiGHS の潜りの分枝 (列=値) を固定して、カットなしの LP の値を highspy で解く。使い方: replay_lp.py 列=値 ..."""
import sys, highspy
h = highspy.Highs(); h.setOptionValue("output_flag", False)
h.readModel(".miplib_cache/neos-2978193-inde.mps")
lp = h.getLp()
for a in sys.argv[1:]:
    j, v = a.split("="); j = int(j); v = float(v)
    h.changeColBounds(j, v, v)
h.setOptionValue("solve_relaxation", True)
h.run()
print(h.getModelStatus(), h.getInfo().objective_function_value)
