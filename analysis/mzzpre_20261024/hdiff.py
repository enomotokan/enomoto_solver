"""HiGHS の前処理で消えた列とその値を調べる: python hdiff.py <mps> <出力.tsv> [キー=値 ...]
前処理後の LP 緩和の解を 2 つ (元の目的・目的 0) 後処理で戻し、消えた列の値が 2 つで同じなら「固定」、違えば「代入」とする。
出力: 列名 種類 値1 値2。最後に元の問題で「固定」だけを当てた LP 緩和の値を出す。"""
import sys, json
import numpy as np
import highspy
mps, out = sys.argv[1], sys.argv[2]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 4)
for kv in sys.argv[3:]:
    k, v = kv.split("=")
    for cast in (int, float, str):
        try:
            if h.setOptionValue(k, cast(v)) == highspy.HighsStatus.kOk: break
        except Exception: pass
h.readModel(mps)
orig = h.getLp()
names = list(orig.col_names_)
h.presolve()
plp = h.getPresolvedLp()
pnames = set(plp.col_names_)
def solve_relax(lp, zero=False):
    q = highspy.Highs(); q.setOptionValue("output_flag", False); q.setOptionValue("threads", 4)
    lp = highspy.HighsLp(lp) if hasattr(highspy, "HighsLp") else lp
    lp.integrality_ = []
    if zero: lp.col_cost_ = np.zeros(lp.num_col_)
    q.passModel(lp); q.run()
    return q.getSolution(), q.getInfo().objective_function_value
X = []
for zero in (False, True):
    s, obj = solve_relax(h.getPresolvedLp(), zero)
    h.postsolve(s)
    X.append(np.array(h.getSolution().col_value))
removed = [j for j, n in enumerate(names) if n not in pnames]
kinds = {}
with open(out, "w") as f:
    for j in removed:
        a, b = X[0][j], X[1][j]
        k = "fix" if abs(a - b) < 1e-9 else "sub"
        kinds[j] = k
        f.write(f"{names[j]}\t{k}\t{a:g}\t{b:g}\n")
# 元の問題で「固定」だけを当てた LP 緩和
lp = h.getLp()
lo = np.array(lp.col_lower_); up = np.array(lp.col_upper_)
for j, k in kinds.items():
    if k == "fix":
        lo[j] = up[j] = round(X[0][j])
lp.col_lower_ = lo; lp.col_upper_ = up
_, v = solve_relax(lp)
print(json.dumps({"opts": sys.argv[3:], "removed": len(removed), "fix": sum(1 for k in kinds.values() if k == "fix"),
                  "fix1": sum(1 for j, k in kinds.items() if k == "fix" and round(X[0][j]) == 1), "lp_orig_with_fix": v}), flush=True)
