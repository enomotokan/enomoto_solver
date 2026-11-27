"""確かめ (b): sat_rens の形の乱数の整数問題を、当方 (試作 ENOMOTO_AGG_DUALFREE_INEQ を使う) と HiGHS で解いて突き合わせる。
python fuzz.py 問題数 [開始seed] [off]  (off: 試作を使わない対照)。当方の解は元の問題で実行可能かも確かめる。規則が当たった数は
ENOMOTO_DEBUG_AGGREGATOR=1 の標準エラーの DEBUG_DUALFREE_INEQ 行で数える。"""
import os, sys, time
if "off" not in sys.argv[1:]:
    os.environ.setdefault("ENOMOTO_AGG_DUALFREE_INEQ", "1")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import highspy
from enomoto_solver import Model
from gen import gen
inf = float('inf')

def highs(lp):
    lo, up, c, it, rows, rlo, rup = lp
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.setOptionValue('mip_rel_gap', 1e-9)
    K = highspy.kHighsInf
    for j in range(len(lo)):
        h.addVar(lo[j] if lo[j] > -inf else -K, up[j] if up[j] < inf else K); h.changeColCost(j, c[j])
        if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in zip(rows, rlo, rup):
        h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
    h.run(); return h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value

def ours(lp):
    lo, up, c, it, rows, rlo, rup = lp
    M = Model(); xs = [M.add_variable(int if it[j] else float, lo[j], up[j]) for j in range(len(lo))]
    M.set_objective(sum(cc * x for cc, x in zip(c, xs)))
    for r, l, u in zip(rows, rlo, rup):
        e = sum(v * xs[j] for j, v in r)
        if l == u: M.add_constraint(e == l)
        else:
            if l > -inf: M.add_constraint(e >= l)
            if u < inf: M.add_constraint(e <= u)
    s = M.solve(mip_rel_gap=1e-9, time_limit=20.0, threads=1, log=False)
    return s.status, s.objective, [x.value for x in xs] if s.status == "optimal" else None

def feasible(lp, x):
    lo, up, c, it, rows, rlo, rup = lp
    for j in range(len(lo)):
        t = 1e-6 * (1 + abs(x[j]))
        if x[j] < lo[j] - t or x[j] > up[j] + t: return f"列 {j} の境界 {x[j]} [{lo[j]}, {up[j]}]"
        if it[j] and abs(x[j] - round(x[j])) > 1e-6: return f"列 {j} が整数でない {x[j]}"
    for i, (r, l, u) in enumerate(zip(rows, rlo, rup)):
        a = sum(v * x[j] for j, v in r); t = 1e-6 * (1 + sum(abs(v * x[j]) for j, v in r))
        if a < l - t or a > u + t: return f"行 {i} の違反 {a} [{l}, {u}]"
    return None

N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time(); cnt = {}
for seed in range(start, start + N):
    lp = gen(seed)
    hs, ho = highs(lp); t1 = time.time(); os_, oo, x = ours(lp); dt = time.time() - t1
    if dt > 5: print("slow seed", seed, "%.1f s" % dt, os_, flush=True)
    hmap = {'Optimal': 'optimal', 'Infeasible': 'infeasible', 'Unbounded': 'unbounded', 'Primal infeasible or unbounded': 'inf_or_unb'}.get(hs, hs)
    cnt[hmap] = cnt.get(hmap, 0) + 1
    ok = (hmap == os_) or (hmap in ('unbounded', 'inf_or_unb') and os_ in ('unbounded', 'infeasible', 'inf_or_unb', 'infeasible_or_unbounded'))
    why = ""
    if ok and hmap == 'optimal':
        if abs(ho - oo) > 1e-6 * (1 + abs(ho)): ok = False; why = "最適値"
        else:
            f = feasible(lp, x)
            if f: ok = False; why = f
    if not ok:
        bad += 1
        if bad <= 15: print("seed", seed, "highs", hs, ho, "ours", os_, oo, why, flush=True)
print("bad", bad, "of", N, "HiGHS の状態", cnt, "time %.1f" % (time.time() - t0))
