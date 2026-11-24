"""x_*.json の整数列を固定し、残りの連続列を HiGHS の LP で解き直す (許容差 1e-9)。
解の目的値が、許容差の中で行を少しずつ破ることで下がっていないかを見る。使い方: python fixlp.py x_*.json ..."""
import json, sys
import numpy as np, highspy
from pathlib import Path
R = Path(__file__).resolve().parents[2]
for f in sys.argv[1:]:
    d = json.loads(Path(f).read_text())
    h = highspy.Highs(); h.setOptionValue("output_flag", False)
    h.readModel(str(R / ".miplib_cache/neos-5188808-nattai.mps"))
    for k, v in (("primal_feasibility_tolerance", 1e-9), ("dual_feasibility_tolerance", 1e-9), ("mip_feasibility_tolerance", 1e-9)):
        h.setOptionValue(k, v)
    lp = h.getLp(); names = list(lp.col_names_); integ = list(lp.integrality_)
    nint = 0
    for j, nm in enumerate(names):
        if int(integ[j]) == 1:
            v = round(d["x"][nm]); h.changeColBounds(j, v, v); nint += 1
    h.setOptionValue("solver", "simplex")
    h.run()
    st = h.modelStatusToString(h.getModelStatus())
    print(f"{f}: 報告 {d['objective']:.10g} / 整数列 {nint} 本を固定した LP (許容差 1e-9): {st} {h.getInfo().objective_function_value:.10g}")
