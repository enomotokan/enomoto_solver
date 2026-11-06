"""書き出したカットの LP (<prefix>.mps/.bas) を HiGHS で、同じ出発の基底 (setBasis) と全スラック基底から解く。
使い方: python highs_basis.py <prefix> [warm|cold] [time]
.bas の 2 行目は基底の変数 (構造列 j < n、スラック n + i)、3 行目は構造列の値 x。非基底の列は x に近い方の限界、
非基底の行は A x に近い方の限界に置く。"""
import sys, time
import numpy as np
import highspy

prefix, mode = sys.argv[1], (sys.argv[2] if len(sys.argv) > 2 else "warm")
tl = float(sys.argv[3]) if len(sys.argv) > 3 else 120.0
h = highspy.Highs()
h.setOptionValue("output_flag", True)
h.readModel(prefix + ".mps")
lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
h.changeColsIntegrality(n, np.arange(n, dtype=np.int32), np.zeros(n, dtype=np.uint8))
for k, v in [("presolve", "off"), ("solver", "simplex"), ("simplex_strategy", 1), ("time_limit", tl), ("random_seed", 0)] + [(k, int(v)) for k, v in (a.split("=") for a in sys.argv[4:])]:
    h.setOptionValue(k, v)
lines = open(prefix + ".bas").read().split("\n")
b = [int(t) for t in lines[1].split()]
x = np.array([float(t) for t in lines[2].split()]) if len(lines) > 2 and lines[2].strip() else np.zeros(n)
lp = h.getLp()
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
rlo, rup = np.array(lp.row_lower_), np.array(lp.row_upper_)
A = lp.a_matrix_
st, idx, val = np.array(A.start_), np.array(A.index_), np.array(A.value_)
ax = np.zeros(m)
for j in range(n):
    s, e = st[j], st[j + 1]
    ax[idx[s:e]] += val[s:e] * x[j]
print(f"n={n} m={m} nnz={len(val)} basis={len(b)}", flush=True)
if mode == "warm":
    B = highspy.HighsBasisStatus
    def nb(v, l, u):
        if l == u:
            return B.kLower
        if np.isinf(l) and np.isinf(u):
            return B.kZero
        if np.isinf(u):
            return B.kLower
        if np.isinf(l):
            return B.kUpper
        return B.kLower if abs(v - l) <= abs(v - u) else B.kUpper
    basic = set(b)
    cs = [B.kBasic if j in basic else nb(x[j], lo[j], up[j]) for j in range(n)]
    rs = [B.kBasic if n + i in basic else nb(ax[i], rlo[i], rup[i]) for i in range(m)]
    hb = highspy.HighsBasis()
    hb.col_status = cs
    hb.row_status = rs
    hb.valid = True
    print("setBasis:", h.setBasis(hb), "basic cols", sum(1 for j in b if j < n), "basic rows", sum(1 for j in b if j >= n), flush=True)
t0 = time.time()
h.run()
dt = time.time() - t0
info = h.getInfo()
print(f"HIGHS mode={mode} status={h.modelStatusToString(h.getModelStatus())} iters={info.simplex_iteration_count} time={dt:.3f}s "
      f"({1e6*dt/max(1,info.simplex_iteration_count):.1f} us/iter) obj={info.objective_function_value:.10e}", flush=True)
# 環境変数 HB_DUMP=<パス>: 終わった時の基底を .bas の 2 行目と同じ形 (構造列 j、スラック n + i) で書き出す
import os
if os.environ.get("HB_DUMP"):
    hb = h.getBasis()
    bb = [j for j in range(n) if hb.col_status[j] == highspy.HighsBasisStatus.kBasic] + [n + i for i in range(m) if hb.row_status[i] == highspy.HighsBasisStatus.kBasic]
    open(os.environ["HB_DUMP"], "w").write(f"{n} {m}\n" + " ".join(map(str, bb)) + "\n")
