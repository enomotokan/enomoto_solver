"""HiGHS の前処理で締まった列の境界を元の問題に当てて LP 緩和を測る: python hbounds.py <mps> <固定の tsv> [キー=値 ...]
残った列の境界は、元の下限が 0 でない整数列は元の下限だけずらしたとみなして元の座標に戻す。
(1) 締まった境界だけ、(2) 固定だけ (hdiff.py の fix)、(3) 両方 の LP 緩和の値を出す。締まった境界の一覧を出す。"""
import sys, json
import numpy as np
import highspy
mps, fixtsv = sys.argv[1], sys.argv[2]
h = highspy.Highs(); h.setOptionValue("output_flag", False)
for kv in sys.argv[3:]:
    k, v = kv.split("="); h.setOptionValue(k, int(v))
h.readModel(mps)
l0 = h.getLp(); lo0 = np.array(l0.col_lower_); up0 = np.array(l0.col_upper_); names = list(l0.col_names_)
n0 = {n: j for j, n in enumerate(names)}
h.presolve(); l1 = h.getPresolvedLp()
tl, tu = lo0.copy(), up0.copy(); tight = []
for k, n in enumerate(l1.col_names_):
    j = n0[n]; s = lo0[j] if (lo0[j] != 0 and np.isfinite(lo0[j]) and l1.col_lower_[k] == 0) else 0.0
    a, b = l1.col_lower_[k] + s, l1.col_upper_[k] + s
    if a > lo0[j] + 1e-9 or b < up0[j] - 1e-9:
        tight.append((n, lo0[j], up0[j], a, b)); tl[j] = max(tl[j], a); tu[j] = min(tu[j], b)
fl, fu = lo0.copy(), up0.copy()
for line in open(fixtsv):
    n, kind, v, _ = line.split("\t")
    if kind == "fix":
        j = n0[n]; fl[j] = fu[j] = round(float(v))
def lpval(lo, up):
    q = highspy.Highs(); q.setOptionValue("output_flag", False); q.readModel(mps)
    lp = q.getLp(); lp.integrality_ = []; lp.col_lower_ = lo; lp.col_upper_ = up; q.passModel(lp); q.run()
    return q.getInfo().objective_function_value, q.getModelStatus()
print("tightened", len(tight), tight[:40])
print(json.dumps({"bounds": str(lpval(tl, tu)), "fix": str(lpval(fl, fu)), "both": str(lpval(np.maximum(tl, fl), np.minimum(tu, fu)))}))
