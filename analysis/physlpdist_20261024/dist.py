"""ENOMOTO の根の LP 解と整数最適解の離れ方: python dist.py <接頭辞k> [最適値] [秒]
<接頭辞k>.mps (ENOMOTO_MIP_DEBUG_DUMP_ROOT_X で書いた問題)、<接頭辞k>_x.txt (カットの後の LP 解)、<接頭辞k>_x0.txt (カットの前)。
(1) LP 解の形 (自由な整数列・分数の列)、(2) HiGHS の最適解 1 つとの差、(3) 目的値 <= 最適値 の整数解のうち LP 解に
最も近いもの (整数列の L1 距離の最小、HiGHS で解く) との差。最適解は <接頭辞k>_opt.txt に書き、あれば読む。"""
import sys, time, os
import numpy as np
import highspy

pre = sys.argv[1]
zopt_arg = float(sys.argv[2]) if len(sys.argv) > 2 and sys.argv[2] else None
T = float(sys.argv[3]) if len(sys.argv) > 3 else 300.0
TOL = 1e-6


def load():
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.readModel(pre + ".mps")
    return h


h = load()
lp = h.getLp()
n = lp.num_col_
c = np.array(lp.col_cost_)
off = lp.offset_
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
integ = np.array([int(v) for v in lp.integrality_]) if len(lp.integrality_) else np.zeros(n, int)
isint = integ == 1
free = isint & (lo < up)
A = h.getLp().a_matrix_
rl, ru = np.array(lp.row_lower_), np.array(lp.row_upper_)
import scipy.sparse as sp
M = sp.csc_matrix((A.value_, A.index_, A.start_), shape=(lp.num_row_, n))


def info(name, x):
    ax = M @ x
    viol = max(0.0, float(np.max(rl - ax)), float(np.max(ax - ru)), float(np.max(lo - x)), float(np.max(x - up)))
    xf = x[free]
    frac = np.abs(xf - np.round(xf)) > TOL
    fl = np.minimum(xf - np.floor(xf), np.ceil(xf) - xf)[frac].sum()
    print(f"[{name}] sum over fractional columns of the distance to the nearest integer {fl:.1f}")
    print(f"[{name}] objective {c @ x + off:.4f}, max violation {viol:.2e}; free integer columns {free.sum()}, "
          f"fractional {frac.sum()} ({frac.mean():.3f}), integral at 0 {np.sum(~frac & (np.round(xf) == 0))}, at 1 {np.sum(~frac & (np.round(xf) == 1))}"
          f"; fractional values: <0.1 {np.sum(frac & (xf < 0.1))}, 0.1-0.9 {np.sum(frac & (xf >= 0.1) & (xf <= 0.9))}, >0.9 {np.sum(frac & (xf > 0.9))}")
    return frac


def cmp(name, x, xs):
    """LP 解 x と整数解 xs の差 (自由な整数列だけ)。"""
    xf, sf = x[free], xs[free]
    frac = np.abs(xf - np.round(xf)) > TOL
    integral = ~frac
    flip = integral & (np.abs(np.round(xf) - sf) > 0.5)
    l1 = np.abs(xf - sf).sum()
    # 分数の列を最も近い整数に丸めたとき整数解と合う割合
    fr_round_ok = np.mean(np.round(xf[frac]) == sf[frac]) if frac.any() else float("nan")
    print(f"  vs {name}: L1 {l1:.1f} (integral part {np.abs(xf - sf)[integral].sum():.1f}, fractional part {np.abs(xf - sf)[frac].sum():.1f}); "
          f"LP-integral columns that differ {flip.sum()} of {integral.sum()} (at 1 in LP -> 0: {np.sum(flip & (np.round(xf) == 1))}, "
          f"0 -> 1: {np.sum(flip & (np.round(xf) == 0))}); fractional rounded = solution {fr_round_ok:.3f}; "
          f"columns at 1 in solution {int(np.sum(sf > 0.5))}")


xs = {k: np.loadtxt(pre + f"_{k}.txt") for k in ["x", "x0"] if os.path.exists(pre + f"_{k}.txt")}
for k, x in xs.items():
    info(k, x)

# (2) HiGHS の最適解
optf = pre + "_opt.txt"
if os.path.exists(optf):
    xopt = np.loadtxt(optf)
    zopt = c @ xopt + off
else:
    g = load()
    g.setOptionValue("threads", 4)
    g.setOptionValue("time_limit", T)
    t = time.perf_counter()
    g.run()
    xopt = np.array(g.getSolution().col_value)
    zopt = g.getInfo().objective_function_value
    print(f"HiGHS: {g.modelStatusToString(g.getModelStatus())} {zopt:.4f} ({time.perf_counter() - t:.1f}s)")
    np.savetxt(optf, xopt)
if zopt_arg is not None:
    zopt = zopt_arg
print(f"optimum used {zopt:.4f}")
for k, x in xs.items():
    print(f"[{k}]")
    cmp("HiGHS optimum", x, xopt)

# (3) 目的値 <= zopt の整数解のうち LP 解に最も近いもの
only = os.environ.get('ONLY')
for k, x in xs.items():
    if only and k != only:
        continue
    g = load()
    g.setOptionValue("threads", 4)
    g.setOptionValue("time_limit", T)
    # 目的値の制約 c x + off <= zopt (+ 1e-4。連続列があり目的値は整数とは限らない)
    idx = np.nonzero(c)[0].astype(np.int32)
    g.addRow(-highspy.kHighsInf, zopt - off + 1e-4, len(idx), idx, c[idx])
    # 新しい目的: 自由な整数列の |x_j - v_j|。0-1 なら v + (1 - 2v) x_j、ほかは補助の列 d >= ±(x_j - v_j)
    newc = np.zeros(n)
    const = 0.0
    gen = []
    for j in np.nonzero(free)[0]:
        v = x[j]
        if lo[j] == 0 and up[j] == 1:
            newc[j] = 1 - 2 * v
            const += v
        else:
            gen.append(j)
    g.changeColsCost(n, np.arange(n, dtype=np.int32), newc)
    for j in gen:
        d = g.getNumCol()
        g.addCol(1.0, 0.0, highspy.kHighsInf, 0, np.array([], np.int32), np.array([]))
        g.addRow(-x[j], highspy.kHighsInf, 2, np.array([d, j], np.int32), np.array([1.0, -1.0]))
        g.addRow(x[j], highspy.kHighsInf, 2, np.array([d, j], np.int32), np.array([1.0, 1.0]))
    g.changeObjectiveOffset(const)
    # 最適解を初期解に
    sol = highspy.HighsSolution()
    sol.col_value = list(xopt) + [abs(xopt[j] - x[j]) for j in gen]
    g.setSolution(sol)
    t = time.perf_counter()
    g.run()
    st = g.modelStatusToString(g.getModelStatus())
    dist = g.getInfo().objective_function_value
    bnd = g.getInfo().mip_dual_bound
    xn = np.array(g.getSolution().col_value)[:n]
    print(f"[{k}] nearest solution with objective <= {zopt:.0f}: {st}, L1 {dist:.2f} (lower bound {bnd:.2f}), objective {c @ xn + off:.4f} ({time.perf_counter() - t:.1f}s)")
    cmp("nearest optimum", x, xn)
    np.savetxt(pre + f"_near_{k}.txt", xn)
