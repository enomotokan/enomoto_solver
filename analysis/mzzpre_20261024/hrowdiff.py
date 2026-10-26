"""元の問題の LP 緩和の最適解 x* を HiGHS の前処理後の問題の行で調べる: python hrowdiff.py <mps> [キー=値 ...]
前処理後の行のうち x* (残った列だけ) が破る行を、元の行と同じ (名前と係数が同じ)・係数が変わった・新しい行に分けて出す。
消えた列で x* が 0 でないものの数も出す。"""
import sys
import numpy as np
import highspy
from scipy.sparse import csc_matrix
mps = sys.argv[1]
fixtsv = sys.argv.pop(2) if len(sys.argv) > 2 and sys.argv[2].endswith(".tsv") else None
def load(h):
    lp = h.getLp() if not isinstance(h, highspy.HighsLp) else h
    a = lp.a_matrix_
    A = csc_matrix((np.array(a.value_), np.array(a.index_), np.array(a.start_)), shape=(lp.num_row_, lp.num_col_)).tocsr()
    return lp, A
h = highspy.Highs(); h.setOptionValue("output_flag", False)
for kv in sys.argv[2:]:
    k, v = kv.split("="); h.setOptionValue(k, int(v))
h.readModel(mps)
lp0, A0 = load(h)
q = highspy.Highs(); q.setOptionValue("output_flag", False); q.readModel(mps); lpr = q.getLp(); lpr.integrality_ = []
if fixtsv:
    nn = {n: j for j, n in enumerate(lpr.col_names_)}; lo = np.array(lpr.col_lower_); up = np.array(lpr.col_upper_)
    for line in open(fixtsv):
        nm, kind, v, _ = line.split("\t")
        if kind == "fix": lo[nn[nm]] = up[nn[nm]] = round(float(v))
    lpr.col_lower_ = lo; lpr.col_upper_ = up
q.passModel(lpr); q.run()
x = np.array(q.getSolution().col_value); print("orig LP", q.getInfo().objective_function_value)
print("presolve", h.presolve(), h.getModelPresolveStatus())
lp1, A1 = load(h.getPresolvedLp())
cn0 = {n: j for j, n in enumerate(lp0.col_names_)}; rn0 = {n: i for i, n in enumerate(lp0.row_names_)}
print("presolved", lp1.num_row_, lp1.num_col_, len(lp1.col_names_))
cmap = np.array([cn0[n] for n in lp1.col_names_], dtype=int)
xr = x[cmap].copy()
lo0c = np.array(lp0.col_lower_)[cmap]; lo1c = np.array(lp1.col_lower_)
sh = (lo0c != 0) & np.isfinite(lo0c) & (lo1c == 0)
xr[sh] -= lo0c[sh]  # 下限を 0 にずらした整数列
kept = set(cmap.tolist())
gone_nz = [(lp0.col_names_[j], x[j]) for j in range(lp0.num_col_) if j not in kept and abs(x[j]) > 1e-9]
print("removed columns nonzero in x*:", len(gone_nz), gone_nz[:20])
act = A1 @ xr
lo, up = np.array(lp1.row_lower_), np.array(lp1.row_upper_)
viol = np.maximum(lo - act, act - up)
cnt = {"same": 0, "changed": 0, "new": 0}
for i in np.argsort(-viol):
    if viol[i] <= 1e-6: break
    n = lp1.row_names_[i]
    r1 = {lp1.col_names_[j]: v for j, v in zip(A1[i].indices, A1[i].data)}
    if n in rn0:
        i0 = rn0[n]; r0 = {lp0.col_names_[j]: v for j, v in zip(A0[i0].indices, A0[i0].data)}
        same = r0 == r1 and lo[i] == lp0.row_lower_[i0] and up[i] == lp0.row_upper_[i0]
        kind = "same" if same else "changed"
        gone = {c: r0[c] for c in r0 if c not in r1}; add = {c: r1[c] for c in r1 if c not in r0}
        chg = {c: (r0[c], r1[c]) for c in r1 if c in r0 and abs(r0[c] - r1[c]) > 1e-9}
        xv = lambda c: round(float(x[cn0[c]]), 3)
        extra = "" if same else (f" orig[{lp0.row_lower_[i0]:g},{lp0.row_upper_[i0]:g}] len {len(r0)}->{len(r1)} gone {[(c, v, xv(c)) for c, v in gone.items()]}"
                                 f" added {[(c, v, xv(c)) for c, v in add.items()]} coef {chg}")
    else:
        kind, extra = "new", ""
    cnt[kind] += 1
    if cnt[kind] <= 12:
        print(f"{kind} {n} viol {viol[i]:.3g} [{lo[i]:g},{up[i]:g}]{extra}")
print(cnt)
