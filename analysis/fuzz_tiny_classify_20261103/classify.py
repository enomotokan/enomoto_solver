"""fuzz_tiny の食い違いを全部分類する。
使い方: python analysis/fuzz_tiny_classify_20261103/classify.py 問題数 [開始seed]  (リポジトリの根から)
各 seed: ENOMOTO・HiGHS の状態と値、両方の解の元の問題での最大違反 (境界・整数性・行、絶対)、1e-9・1e-6 で実行可能か、
HiGHS に ENOMOTO の解を固定して与えた判定 (全列固定 / 整数列だけ固定)、HiGHS を厳しい許容誤差で解いた値、
厳密な最適値 (2 値の全列挙 + 連続列を 1e9 倍した LP) と許容誤差 1e-6 の内の最適値 (同じ全列挙で行を 1e-6 緩めたもの)。"""
import sys, json, itertools, highspy
from fractions import Fraction as F
sys.argv = ["fuzz_tiny.py", "0"] + sys.argv[1:]
FT = "benchmarks/equiv2_bug_20261022/fuzz_tiny.py"
src = open(FT).read().split("N = int(sys.argv[1])")[0]
ns = {"__file__": FT}; exec(compile(src, FT, "exec"), ns)
gen, highs, ours = ns["gen"], ns["highs"], ns["ours"]
K = highspy.kHighsInf

def build(lp, lo=None, up=None, opts=None, integ=True):
    lo0, up0, c, it, rows, rlo, rup = lp
    lo = lo or lo0; up = up or up0
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("mip_rel_gap", 1e-9)
    for k, v in (opts or {}).items(): h.setOptionValue(k, v)
    for j in range(len(lo)):
        h.addVar(lo[j], up[j]); h.changeColCost(j, c[j])
        if integ and it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in zip(rows, rlo, rup):
        h.addRow(l if l > -1e300 else -K, u, len(r), [a for a, _ in r], [b for _, b in r])
    h.run()
    st = h.modelStatusToString(h.getModelStatus())
    x = list(h.getSolution().col_value) if st == "Optimal" else None
    return st, (h.getInfo().objective_function_value if x is not None else None), x

def viol(lp, x):
    lo, up, c, it, rows, rlo, rup = lp
    vb = max(max(lo[j] - x[j], x[j] - up[j], 0) for j in range(len(x)))
    vi = max([abs(x[j] - round(x[j])) for j in range(len(x)) if it[j]] or [0])
    vr = max(max(rlo[i] - sum(a * x[j] for j, a in r), sum(a * x[j] for j, a in r) - rup[i], 0) for i, r in enumerate(rows))
    return vb, vi, vr

def best_by_enum(lp, rowtol):
    """2 値を全列挙し、各点で連続列 y = 1e-9 z の LP (z は整数の係数、良い条件) を解く。rowtol=0 で厳密、1e-6 で許容誤差の内
    (行の右辺を 1e-6 だけ緩める。境界は緩めない)。返り値: (最適値, 2 値の点)。"""
    lo, up, c, it, rows, rlo, rup = lp
    nb = sum(it); ys = [j for j in range(len(lo)) if not it[j]]
    best = (None, None)
    for xb in itertools.product((0, 1), repeat=nb):
        # 行: sum_x a x + 1e-9 * sum_y a z  in [rlo - tol, rup + tol] → z の行 (正確に分数で右辺を出す)
        h = highspy.Highs(); h.setOptionValue("output_flag", False)
        for j in ys: h.addVar(0.0, float(F(up[j]) / F(1, 10**9))); h.changeColCost(ys.index(j), 0.0)
        ok = True
        for r, l, u in zip(rows, rlo, rup):
            ax = sum(F(a) * xb[j] for j, a in r if j < nb); zr = [(ys.index(j), a) for j, a in r if j >= nb]
            tol = F(rowtol)
            # F(1e-9) は 1e-9 の 2 進の近似。z の右辺は (b - ax) / 1e-9 を浮動小数で出す (z の LP の許容誤差 1e-7 は y で 1e-16)
            ub = None if u == float("inf") else float((F(u) + tol - ax) / F(1e-9))
            lb = None if l == -float("inf") else float((F(l) - tol - ax) / F(1e-9))
            if not zr:
                if (ub is not None and ub < 0) or (lb is not None and lb > 0): ok = False; break
                continue
            h.addRow(lb if lb is not None else -K, ub if ub is not None else K, len(zr), [a for a, _ in zr], [b for _, b in zr])
        if not ok: continue
        for k, j in enumerate(ys): h.changeColCost(k, c[j] * 1e-9)
        h.run()
        if h.modelStatusToString(h.getModelStatus()) != "Optimal": continue
        z = sum(c[j] * xb[j] for j in range(nb)) + h.getInfo().objective_function_value
        if best[0] is None or z < best[0]: best = (z, xb)
    return best

N = int(sys.argv[2]); start = int(sys.argv[3]) if len(sys.argv) > 3 else 0
TIGHT = {"primal_feasibility_tolerance": 1e-10, "mip_feasibility_tolerance": 1e-10, "presolve": "off"}
for seed in range(start, start + N):
    lp = gen(seed); hs, ho = highs(lp); os_, oo, ox = ours(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible"}.get(hs, hs)
    if (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho))): continue
    lo, up, c, it, rows, rlo, rup = lp
    _, _, hx = build(lp)
    d = dict(seed=seed, highs_status=hs, highs_obj=ho, ours_status=os_, ours_obj=oo)
    for name, x in (("highs", hx), ("ours", ox)):
        if x is None: continue
        vb, vi, vr = viol(lp, x); m = max(vb, vi, vr)
        d[name + "_viol"] = [vb, vi, vr]; d[name + "_feas_1e-9"] = m <= 1e-9; d[name + "_feas_1e-6"] = m <= 1e-6
    if ox is not None:
        # 全列を ENOMOTO の値に固定 (HiGHS が許容誤差の内で受けるか)
        st, zf, _ = build(lp, lo=list(ox), up=list(ox), integ=False)
        d["fix_all_in_highs"] = [st, zf]
        # 整数列だけ固定 (その整数の点で HiGHS が連続列を選ぶ)
        xi = [round(v) for v in ox]
        lo2 = [float(xi[j]) if it[j] else lo[j] for j in range(len(lo))]; up2 = [float(xi[j]) if it[j] else up[j] for j in range(len(lo))]
        st, zf, _ = build(lp, lo=lo2, up=up2)
        d["fix_int_in_highs"] = [st, zf]
        d["ours_int_point"] = [xi[j] for j in range(len(lo)) if it[j]]
    if hx is not None: d["highs_int_point"] = [round(v) for v, t in zip(hx, it) if t]
    st, zt, _ = build(lp, opts=TIGHT); d["highs_tight"] = [st, zt]
    ze, xe = best_by_enum(lp, 0.0); zt6, xt6 = best_by_enum(lp, 1e-6)
    d["exact_opt"] = ze; d["exact_int_point"] = xe; d["tol6_opt"] = zt6; d["tol6_int_point"] = xt6
    print(json.dumps(d), flush=True)
