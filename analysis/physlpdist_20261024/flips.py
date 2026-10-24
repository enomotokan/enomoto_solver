"""LP で整数値の列 (自由な整数列のうち LP 解で 0 か 1) を、実行可能解にするために最低いくつ反転させる必要があるか:
python flips.py <接頭辞k> <目的値の上限 or ''> [秒]。分数の列は自由。RENS (整数値の列を全部固定) の近傍に解がないことの度合い。"""
import sys, time
import numpy as np
import highspy

pre = sys.argv[1]
zmax = float(sys.argv[2]) if len(sys.argv) > 2 and sys.argv[2] else None
T = float(sys.argv[3]) if len(sys.argv) > 3 else 300.0
g = highspy.Highs()
g.setOptionValue("output_flag", False)
g.setOptionValue("threads", 4)
g.setOptionValue("time_limit", T)
g.readModel(pre + ".mps")
lp = g.getLp()
n = lp.num_col_
c = np.array(lp.col_cost_)
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
isint = np.array([int(v) for v in lp.integrality_]) == 1
x = np.loadtxt(pre + "_x.txt")
free = isint & (lo < up)
integral = free & (np.abs(x - np.round(x)) <= 1e-6) & (lo == 0) & (up == 1)
if zmax is not None:
    idx = np.nonzero(c)[0].astype(np.int32)
    g.addRow(-highspy.kHighsInf, zmax - lp.offset_ + 1e-4, len(idx), idx, c[idx])
newc = np.zeros(n)
const = 0.0
for j in np.nonzero(integral)[0]:
    v = round(x[j])
    newc[j] = 1.0 if v == 0 else -1.0
    const += v
g.changeColsCost(n, np.arange(n, dtype=np.int32), newc)
g.changeObjectiveOffset(const)
t = time.perf_counter()
g.run()
st = g.modelStatusToString(g.getModelStatus())
info = g.getInfo()
xs = np.array(g.getSolution().col_value)
z = c @ xs + lp.offset_ if info.primal_solution_status else float("nan")
print(f"{pre}: objective bound {zmax}: {st}, flips of LP-integral columns {info.objective_function_value:.0f} (lower bound {info.mip_dual_bound:.1f}) "
      f"of {integral.sum()}, solution objective {z:.1f} ({time.perf_counter() - t:.1f}s)")
# 反転した列 (列の名前・LP の値・解の値・目的の係数・列の非零の数)
names = lp.col_names_
fl = [j for j in np.nonzero(integral)[0] if abs(xs[j] - round(x[j])) > 0.5]
for j in fl[:40]:
    print(f"  flipped {names[j] if names else j}: LP {x[j]:.0f} -> {xs[j]:.0f}, cost {c[j]:g}, nnz {lp.a_matrix_.start_[j + 1] - lp.a_matrix_.start_[j]}")
np.savetxt(pre + f"_flips_{zmax}.txt", xs)
