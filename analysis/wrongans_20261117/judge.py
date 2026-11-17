"""既定の経路で HiGHS と食い違う乱数問題 (analysis/dualfree_20261117/gen.py) の、どちらが正しいかを確かめる。
python judge.py seed ... [--env]  (--env なしでは試作 ENOMOTO_AGG_DUALFREE_INEQ を外す。--env では環境変数のまま)。HiGHS の解を元の問題で確かめて目的値を計算し、当方の解も同じく確かめる。"""
import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../dualfree_20261117"))
if "--env" not in sys.argv: os.environ.pop("ENOMOTO_AGG_DUALFREE_INEQ", None)
import highspy
from gen import gen
from enomoto_solver import Model, Variable
inf = float('inf')

def viol(lp, x):
    lo, up, c, it, rows, rlo, rup = lp
    w = 0.0; where = None
    for j in range(len(lo)):
        v = max(lo[j] - x[j], x[j] - up[j], abs(x[j] - round(x[j])) if it[j] else 0.0)
        if v > w: w, where = v, f"列 {j}"
    for i, (r, l, u) in enumerate(zip(rows, rlo, rup)):
        a = sum(v * x[j] for j, v in r)
        v = max(l - a, a - u)
        if v > w: w, where = v, f"行 {i}"
    return w, where

def highs(lp, fix=None):
    lo, up, c, it, rows, rlo, rup = lp
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.setOptionValue('mip_rel_gap', 1e-9)
    K = highspy.kHighsInf
    for j in range(len(lo)):
        l, u = lo[j], up[j]
        if fix and j in fix: l = u = fix[j]
        h.addVar(l if l > -inf else -K, u if u < inf else K); h.changeColCost(j, c[j])
        if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in zip(rows, rlo, rup):
        h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
    h.run()
    return h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value, list(h.getSolution().col_value)

def ours(lp):
    lo, up, c, it, rows, rlo, rup = lp
    M = Model(); xs = [Variable(int if it[j] else float, lo[j], up[j]) for j in range(len(lo))]
    M.set_objective(sum(cc * x for cc, x in zip(c, xs)))
    for r, l, u in zip(rows, rlo, rup):
        e = sum(v * xs[j] for j, v in r)
        if l == u: M.add_constraint(e == l)
        else:
            if l > -inf: M.add_constraint(e >= l)
            if u < inf: M.add_constraint(e <= u)
    s = M.solve(mip_rel_gap=1e-9, time_limit=20.0, threads=1, log=False)
    return s.status, s.objective, [x.value for x in xs] if s.status == "optimal" else None

if __name__ == "__main__":
    for seed in [int(a) for a in sys.argv[1:] if a != "--env"]:
        lp = gen(seed); lo, up, c, it = lp[:4]
        hs, ho, hx = highs(lp)
        os_, oo, ox = ours(lp)
        hv = viol(lp, hx) if hs == 'Optimal' else None
        ov = viol(lp, ox) if ox else None
        hobj = sum(cc * v for cc, v in zip(c, hx)) if hs == 'Optimal' else None
        print(f"seed {seed}: n={len(lo)} m={len(lp[4])} | HiGHS {hs} {ho} 違反 {hv} | 当方 {os_} {oo} 違反 {ov}")
        # 当方の整数解を固定して HiGHS で連続部分を解く (当方の整数の選びが HiGHS でどの値になるか)
        if ox:
            fx = {j: round(ox[j]) for j in range(len(lo)) if it[j]}
            print("   当方の整数を固定した HiGHS:", highs(lp, fx)[:2])
        if hs == 'Optimal':
            fx = {j: round(hx[j]) for j in range(len(lo)) if it[j]}
            print("   HiGHS の整数を固定した HiGHS:", highs(lp, fx)[:2])
