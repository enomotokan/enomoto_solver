"""近傍の MPS に HiGHS のカットプールを足した LP を調べる: python pool.py <mps> <pool.json> <打ち切り値> [ENOMOTO の LP 解] [秒]
(1) カットなし・HiGHS のカットありの LP 解の形 (分数の列・0.1〜0.9)、(2) LP で整数値の列を全部固定した近傍 (RENS) が
実行可能か、(3) LP で整数値の列のうち反転が要る最小数 (目的値 <= 打ち切り値)。ENOMOTO の LP 解 (列の順に 1 行 1 値) を渡すと同じことをする。
HiGHS のカットを足した問題を <mps の頭>_hcuts.mps に書く。"""
import sys, json, time, os
import numpy as np
import highspy

mps, poolf, zcut = sys.argv[1], sys.argv[2], float(sys.argv[3])
enox = sys.argv[4] if len(sys.argv) > 4 and sys.argv[4] else None
T = float(sys.argv[5]) if len(sys.argv) > 5 else 60.0


def load(with_cuts):
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.readModel(mps)
    if with_cuts:
        P = json.load(open(poolf))
        st = P["start"]
        for k in range(len(st) - 1):
            idx = np.array(P["index"][st[k]:st[k + 1]], np.int32)
            val = np.array(P["value"][st[k]:st[k + 1]])
            h.addRow(P["lower"][k], P["upper"][k], len(idx), idx, val)
    return h


h0 = load(False)
lp = h0.getLp()
n = lp.num_col_
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
c = np.array(lp.col_cost_)
isint = np.array([int(v) for v in lp.integrality_]) == 1
free = isint & (lo < up)
hc = load(True)
hc.writeModel(os.path.splitext(mps)[0] + "_hcuts.mps")


def lpsol(h):
    g = highspy.Highs()
    g.setOptionValue("output_flag", False)
    g.passModel(h.getLp())
    m = g.getLp().num_col_
    g.changeColsIntegrality(m, np.arange(m, dtype=np.int32), np.zeros(m, dtype=np.uint8))
    g.run()
    return np.array(g.getSolution().col_value), g.getInfo().objective_function_value


def shape(name, x, z):
    xf = x[free]
    frac = np.abs(xf - np.round(xf)) > 1e-6
    print(f"[{name}] LP {z:.4f}: free integer {free.sum()}, fractional {frac.sum()} (0.1-0.9: {np.sum(frac & (xf > 0.1) & (xf < 0.9))})", flush=True)


def rens(name, x):
    """LP で整数値の列を全部固定し、分数の列を [floor, ceil] に (元の行だけ、打ち切り値つき)。"""
    g = load(False)
    g.setOptionValue("time_limit", T)
    g.setOptionValue("objective_bound", zcut)
    integral = free & (np.abs(x - np.round(x)) <= 1e-6)
    nl, nu = lo.copy(), up.copy()
    nl[integral] = np.round(x[integral])
    nu[integral] = np.round(x[integral])
    g.changeColsBounds(n, np.arange(n, dtype=np.int32), nl, nu)
    t = time.perf_counter()
    g.run()
    inf = g.getInfo()
    print(f"[{name}] RENS (all LP-integral fixed, {integral.sum()}): {g.modelStatusToString(g.getModelStatus())}, objective {inf.objective_function_value if inf.primal_solution_status else None} ({time.perf_counter() - t:.1f}s)", flush=True)


def flips(name, x):
    g = load(False)
    g.setOptionValue("time_limit", T)
    integral = free & (np.abs(x - np.round(x)) <= 1e-6) & (lo == 0) & (up == 1)
    idx = np.nonzero(c)[0].astype(np.int32)
    g.addRow(-highspy.kHighsInf, zcut - lp.offset_, len(idx), idx, c[idx])
    newc = np.zeros(n)
    const = 0.0
    for j in np.nonzero(integral)[0]:
        newc[j] = 1.0 if round(x[j]) == 0 else -1.0
        const += round(x[j])
    g.changeColsCost(n, np.arange(n, dtype=np.int32), newc)
    g.changeObjectiveOffset(const)
    t = time.perf_counter()
    g.run()
    inf = g.getInfo()
    print(f"[{name}] minimum flips of LP-integral columns for a solution with objective <= {zcut:g}: {inf.objective_function_value:.0f} (bound {inf.mip_dual_bound:.1f}, {g.modelStatusToString(g.getModelStatus())}, {time.perf_counter() - t:.1f}s)", flush=True)


cases = []
x0, z0 = lpsol(h0)
cases.append(("no cuts", x0[:n], z0))
xh, zh = lpsol(hc)
cases.append(("HiGHS cuts", xh[:n], zh))
np.savetxt(os.path.splitext(mps)[0] + "_hx.txt", xh[:n])
if enox:
    xe = np.loadtxt(enox)
    cases.append(("ENOMOTO cuts", xe, c @ xe + lp.offset_))
for name, x, z in cases:
    shape(name, x, z)
for name, x, z in cases:
    rens(name, x)
    flips(name, x)
